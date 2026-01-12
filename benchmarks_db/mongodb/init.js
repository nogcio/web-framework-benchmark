db = db.getSiblingDB('hello_world');

db.createCollection("users");
db.createCollection("posts");

db.users.createIndex({ email: 1 }, { unique: true });
db.posts.createIndex({ views: -1 });
db.posts.createIndex({ user_id: 1, created_at: -1 });

const users = [];
const posts = [];

for (let i = 1; i <= 10000; i++) {
    const userId = new ObjectId();
    users.push({
        _id: userId,
        username: `user_${i}`,
        email: `user_${i}@example.com`,
        created_at: new Date(),
        last_login: null,
        settings: { theme: "dark", notifications: true, language: "en" }
    });

    for (let j = 1; j <= 15; j++) {
        posts.push({
            _id: new ObjectId(),
            user_id: userId,
            title: `Post ${j}`,
            content: "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",
            views: Math.floor(Math.random() * 10000),
            created_at: new Date(Date.now() - j * 60000)
        });
    }

    if (users.length >= 1000) {
        db.users.insertMany(users);
        users.length = 0;
    }
    if (posts.length >= 1000) {
        db.posts.insertMany(posts);
        posts.length = 0;
    }
}

if (users.length > 0) {
    db.users.insertMany(users);
}
if (posts.length > 0) {
    db.posts.insertMany(posts);
}
