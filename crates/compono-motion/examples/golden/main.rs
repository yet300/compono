//! Prints the golden table of every preset.
//!
//! ```bash
//! cargo run -p compono-motion --example golden > crates/compono-motion/golden/motion.csv
//! ```

mod table;

fn main() {
    print!("{}", table::table());
}
