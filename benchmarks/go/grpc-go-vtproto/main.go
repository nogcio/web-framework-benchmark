package main

import (
	"context"
	"fmt"
	"log"
	"net"
	"os"
	"sync"
    
	_ "github.com/planetscale/vtprotobuf/codec/grpc"
	pb "wfb-go-grpc-vtproto/proto"

	"google.golang.org/grpc"
	"google.golang.org/grpc/health"
	"google.golang.org/grpc/health/grpc_health_v1"
	"google.golang.org/grpc/metadata"
)

// Object pooling to reduce GC pressure
var statePool = sync.Pool{
	New: func() any {
		return &aggregationState{
			amountByCountry:    make(map[string]int64, 4),
			quantityByCategory: make(map[string]int32, 4),
		}
	},
}

type aggregationState struct {
	amountByCountry    map[string]int64
	quantityByCategory map[string]int32
}

func (s *aggregationState) reset() {
	clear(s.amountByCountry)
	clear(s.quantityByCategory)
}

type server struct {
	pb.UnimplementedAnalyticsServiceServer
}

func (s *server) AggregateOrders(ctx context.Context, req *pb.AnalyticsRequest) (*pb.AggregateResult, error) {
	var processedOrders int32
	// Get state from pool
	state := statePool.Get().(*aggregationState)
	defer func() {
		state.reset()
		statePool.Put(state)
	}()

	amountByCountry := state.amountByCountry
	quantityByCategory := state.quantityByCategory

	// Read metadata
	var clientID string
	md, ok := metadata.FromIncomingContext(ctx)
	if ok {
		if vals := md.Get("x-client-id"); len(vals) > 0 {
			clientID = vals[0]
		}
	}

	for _, order := range req.Orders {
		if order.Status == pb.OrderStatus_COMPLETED {
			processedOrders++
			
			var orderAmount int64
			for _, item := range order.Items {
				itemTotal := item.PriceCents * int64(item.Quantity)
				orderAmount += itemTotal
				
				quantityByCategory[item.Category] += item.Quantity
			}
			amountByCountry[order.Country] += orderAmount
		}
	}
    
    // Copy maps
    resAmount := make(map[string]int64, len(amountByCountry))
    for k, v := range amountByCountry {
        resAmount[k] = v
    }
    
    resQty := make(map[string]int32, len(quantityByCategory))
    for k, v := range quantityByCategory {
        resQty[k] = v
    }

	return &pb.AggregateResult{
		ProcessedOrders:    processedOrders,
		AmountByCountry:    resAmount,
		QuantityByCategory: resQty,
		EchoedClientId:     clientID,
	}, nil
}

func main() {
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	lis, err := net.Listen("tcp", fmt.Sprintf(":%s", port))
	if err != nil {
		log.Fatalf("failed to listen: %v", err)
	}
	
	s := grpc.NewServer(
		grpc.MaxConcurrentStreams(256), 
		grpc.InitialWindowSize(1*1024*1024), 
		grpc.InitialConnWindowSize(10*1024*1024), 
	)
	
	pb.RegisterAnalyticsServiceServer(s, &server{})
    
    healthServer := health.NewServer()
    grpc_health_v1.RegisterHealthServer(s, healthServer)
    healthServer.SetServingStatus("", grpc_health_v1.HealthCheckResponse_SERVING)
    healthServer.SetServingStatus("AnalyticsService", grpc_health_v1.HealthCheckResponse_SERVING)

	fmt.Printf("server listening at %v\n", lis.Addr())
	if err := s.Serve(lis); err != nil {
		log.Fatalf("failed to serve: %v", err)
	}
}
