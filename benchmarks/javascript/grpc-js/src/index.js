const grpc = require('@grpc/grpc-js');
const protoLoader = require('@grpc/proto-loader');
const path = require('path');
const cluster = require('cluster');
const os = require('os');
const { HealthImplementation } = require('grpc-health-check');

const PROTO_PATH = path.join(__dirname, '../proto/analytics.proto');
const packageDefinition = protoLoader.loadSync(PROTO_PATH, {
    keepCase: true,
    longs: Number,
    enums: String,
    defaults: true,
    oneofs: true
});
const analyticsProto = grpc.loadPackageDefinition(packageDefinition);

function aggregateOrders(call, callback) {
    let processedOrders = 0;
    const amountByCountry = new Map();
    const quantityByCategory = new Map();
    
    // Header
    const clientIdMap = call.metadata.get('x-client-id');
    const clientId = clientIdMap.length > 0 ? clientIdMap[0] : "";

    const request = call.request;
    const orders = request.orders || [];

    for (let i = 0; i < orders.length; i++) {
        const order = orders[i];
        if (order.status === 'COMPLETED') { 
            processedOrders++;
            
            let orderTotal = 0;

            const items = order.items;
            for (let j = 0; j < items.length; j++) {
                const item = items[j];
                const qty = item.quantity;
                const price = item.price_cents;

                orderTotal += price * qty;

                const cat = item.category;
                const prevQty = quantityByCategory.get(cat);
                quantityByCategory.set(cat, prevQty === undefined ? qty : prevQty + qty);
            }
             
            const country = order.country;
            const prevAmount = amountByCountry.get(country);
            amountByCountry.set(country, prevAmount === undefined ? orderTotal : prevAmount + orderTotal);
        }
    }

    const amountByCountryObj = Object.create(null);
    for (const [key, value] of amountByCountry) amountByCountryObj[key] = value;
    const quantityByCategoryObj = Object.create(null);
    for (const [key, value] of quantityByCategory) quantityByCategoryObj[key] = value;

    callback(null, {
        processed_orders: processedOrders,
        amount_by_country: amountByCountryObj,
        quantity_by_category: quantityByCategoryObj,
        echoed_client_id: clientId
    });
}

function main() {
    const server = new grpc.Server({
        'grpc.max_concurrent_streams': 256,
        'grpc.initial_window_size': 1 * 1024 * 1024, // 1MB
        'grpc.initial_conn_window_size': 10 * 1024 * 1024, // 10MB
        'grpc.max_receive_message_length': 10 * 1024 * 1024,
        'grpc.max_send_message_length': 10 * 1024 * 1024
    });
    
    server.addService(analyticsProto.AnalyticsService.service, { aggregateOrders: aggregateOrders });
    
    // Health check
    const statusMap = {
        "": "SERVING",
        "AnalyticsService": "SERVING"
    };
    const healthImpl = new HealthImplementation(statusMap);
    healthImpl.addToServer(server);
    
    const port = process.env.PORT || '8080';
    server.bindAsync(`0.0.0.0:${port}`, grpc.ServerCredentials.createInsecure(), (err, boundPort) => {
        if (err) {
            console.error(err);
            return;
        }
        console.log(`Server listening on ${boundPort}`);
    });
}

if (cluster.isPrimary) {
    const numCPUs = os.cpus().length;
    for (let i = 0; i < numCPUs; i++) {
        cluster.fork();
    }
    
    cluster.on('exit', (worker, code, signal) => {
        console.log(`worker ${worker.process.pid} died`);
    });
} else {
    main();
}
