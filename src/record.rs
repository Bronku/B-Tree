use rand::Rng;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
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

    pub fn to_text(&self) -> String {
        format!(
            "{},{},{},{},{},{},{}",
            self.key, self.a[0], self.a[1], self.a[2], self.a[3], self.a[4], self.x
        )
    }

    pub fn from_text(s: &str) -> Self {
        let parts: Vec<i32> = s.split(',').map(|p| p.parse().unwrap()).collect();

        Self {
            key: parts[0],
            a: [parts[1], parts[2], parts[3], parts[4], parts[5]],
            x: parts[6],
        }
    }
}
