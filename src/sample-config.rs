pub const HOST: [u8; 4] = [127, 0, 0, 1];
pub const DOMAIN: &str = "example.com";
pub const CERT: (&str, &str) = ("fullchain.pem", "privkey.pem");
pub const RATELIMIT: u32 = 5; // Maximum number of requests per second
pub const COOLDOWN: u64 = 5; // Cooldown period after limit exceeded
pub const PRUNE_TIME: u64 = 3600; // Inactive period until ratelimit entry is pruned
pub const ENDPOINT: &str = "https://api.binance.com";
