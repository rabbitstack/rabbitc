pub mod bridge;
mod netlink;

use rand;
use rand::Rng;

pub fn generate_ifname(len: usize) -> String {
   rand::thread_rng()
        .gen_ascii_chars()
        .take(len)
        .collect::<String>()
}
