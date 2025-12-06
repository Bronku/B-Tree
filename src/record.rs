use bincode::{Decode, Encode};
use rand::Rng;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Encode, Decode)]
pub struct Record {
    pub key: i32,
    pub a: [i32; 5],
    pub x: i32,
}

impl Record {
    pub fn new(a: [i32; 5], x: i32) -> Self {
        let mut key: i32 = 0;
        let mut x_n: i32 = 1;
        for i in 0..5 {
            // rust is annyoing like that, and panics on overflow in arithmetic operations
            key = key.wrapping_add(a[i].wrapping_mul(x_n));
            x_n = x_n.wrapping_mul(x);
        }

        Self { key, a, x }
    }

    pub fn random() -> Self {
        let mut rng = rand::rng();
        let a = rng.random::<[i32; 5]>();
        let x = rng.random::<i32>();
        Self::new(a, x)
    }
}
