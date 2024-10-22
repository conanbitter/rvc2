pub struct Plane {
    pub data: Vec<f64>,
    width: u32,
    height: u32,
}

impl Plane {
    pub fn new(width: u32, height: u32) -> Plane {
        Plane {
            data: vec![0.0; (width * height) as usize],
            width,
            height,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn put(&mut self, x: u32, y: u32, value: f64) {
        self.data[(x + y * self.width) as usize] = value;
    }

    pub fn add(&mut self, x: u32, y: u32, value: f64) {
        self.data[(x + y * self.width) as usize] += value;
    }

    pub fn fill(&mut self, value: f64) {
        for d in self.data.iter_mut() {
            *d = value;
        }
    }

    pub fn scale(&mut self, value: f64) {
        for d in self.data.iter_mut() {
            *d *= value;
        }
    }

    pub fn get(&self, x: u32, y: u32) -> f64 {
        self.data[(x + y * self.width) as usize]
    }
}
