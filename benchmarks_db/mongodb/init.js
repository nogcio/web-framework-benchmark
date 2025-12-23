db = db.getSiblingDB('benchmark');

function getObjectId(i) {
  var hex = i.toString(16);
  while (hex.length < 24) {
    hex = '0' + hex;
  }
  return ObjectId(hex);
}

// Simple LCG for deterministic random numbers
var _seed = 12345;
function seededRandom() {
  var x = Math.sin(_seed++) * 10000;
  return x - Math.floor(x);
}

function randomInt(min, max) {
  return Math.floor(seededRandom() * (max - min + 1)) + min;
}

var baseDate = new Date("2024-01-01T00:00:00Z");

// hello_world
db.createCollection('hello_world');
var bulk = db.hello_world.initializeUnorderedBulkOp();
for (var i = 1; i <= 1000; i++) {
  bulk.insert({
    _id: getObjectId(i),
    name: 'name_' + i,
    created_at: new Date(baseDate.getTime() - i * 1000),
    updated_at: new Date(baseDate.getTime() - (i - 1) * 1000)
  });
}
bulk.execute();

// users
db.createCollection('users');
bulk = db.users.initializeUnorderedBulkOp();
for (var i = 1; i <= 1000; i++) {
  bulk.insert({
    _id: getObjectId(i),
    username: 'user_' + i,
    password_hash: 'hash_' + i
  });
}
bulk.execute();
db.users.createIndex({ username: 1 }, { unique: true });

// tweets
db.createCollection('tweets');
bulk = db.tweets.initializeUnorderedBulkOp();
for (var i = 1; i <= 10000; i++) {
  bulk.insert({
    _id: getObjectId(i),
    user_id: getObjectId(randomInt(1, 1000)),
    content: 'Tweet content ' + i,
    created_at: new Date(baseDate.getTime() - i * 1000)
  });
}
bulk.execute();
db.tweets.createIndex({ created_at: -1 });

// likes
db.createCollection('likes');
db.likes.createIndex({ user_id: 1, tweet_id: 1 }, { unique: true });
db.likes.createIndex({ tweet_id: 1 });
