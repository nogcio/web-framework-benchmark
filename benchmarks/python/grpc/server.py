import asyncio
import os
import multiprocessing
import uvloop

import grpc
from grpc_health.v1 import health, health_pb2, health_pb2_grpc


import analytics_pb2
import analytics_pb2_grpc

STREAM_WINDOW_SIZE = 1 * 1024 * 1024  # 1MB
CONNECTION_WINDOW_SIZE = 10 * 1024 * 1024  # 10MB
MAX_CONCURRENT_STREAMS = 256


class AnalyticsService(analytics_pb2_grpc.AnalyticsServiceServicer):
    async def AggregateOrders(
        self,
        request: analytics_pb2.AnalyticsRequest,
        context: grpc.aio.ServicerContext,
    ) -> analytics_pb2.AggregateResult:
        processed_orders = 0
        amount_by_country: dict[str, int] = {}
        quantity_by_category: dict[str, int] = {}

        client_id = ""
        for key, value in context.invocation_metadata():
            if key.lower() == "x-client-id":
                client_id = value
                break

        for order in request.orders:
            if order.status == analytics_pb2.OrderStatus.COMPLETED:
                processed_orders += 1
                order_amount = 0
                for item in order.items:
                    item_total = item.price_cents * item.quantity
                    order_amount += item_total
                    quantity_by_category[item.category] = (
                        quantity_by_category.get(item.category, 0) + item.quantity
                    )
                amount_by_country[order.country] = (
                    amount_by_country.get(order.country, 0) + order_amount
                )

        result = analytics_pb2.AggregateResult(
            processed_orders=processed_orders,
            echoed_client_id=client_id,
        )
        result.amount_by_country.update(amount_by_country)
        result.quantity_by_category.update(quantity_by_category)
        return result


def build_server() -> grpc.aio.Server:
    options = [
        ("grpc.max_concurrent_streams", MAX_CONCURRENT_STREAMS),
        ("grpc.http2.stream_window_size", STREAM_WINDOW_SIZE),
        ("grpc.http2.connection_window_size", CONNECTION_WINDOW_SIZE),
        ("grpc.so_reuseport", 1),
    ]
    server = grpc.aio.server(options=options)
    analytics_pb2_grpc.add_AnalyticsServiceServicer_to_server(
        AnalyticsService(), server
    )

    health_servicer = health.HealthServicer()
    health_pb2_grpc.add_HealthServicer_to_server(health_servicer, server)
    health_servicer.set("", health_pb2.HealthCheckResponse.SERVING)
    health_servicer.set("AnalyticsService", health_pb2.HealthCheckResponse.SERVING)
    return server


async def serve() -> None:
    port = os.environ.get("PORT", "8080")
    server = build_server()
    server.add_insecure_port(f"0.0.0.0:{port}")
    await server.start()
    print(f"Server listening on 0.0.0.0:{port}")
    await server.wait_for_termination()


def _run_server():
    uvloop.install()
    asyncio.run(serve())


if __name__ == "__main__":
    workers = []
    # Use scheduler affinity to determine real CPU count available in Docker container
    try:
        cpu_count = len(os.sched_getaffinity(0))
    except AttributeError:
        # Fallback for macOS/Windows where sched_getaffinity is not available
        cpu_count = multiprocessing.cpu_count()

    print(f"Starting {cpu_count} workers...")
    for _ in range(cpu_count):
        p = multiprocessing.Process(target=_run_server)
        p.start()
        workers.append(p)
    
    for p in workers:
        p.join()
