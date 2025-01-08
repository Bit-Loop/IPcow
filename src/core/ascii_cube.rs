use std::f32::consts::PI;
use std::thread;
use std::time::Duration;
use std::io::{stdout, Write};
use std::thread::sleep;
use terminal_size::{Width, Height, terminal_size};

const CUBE_VERTICES: [[f32; 3]; 8] = [
    [-1.0, -1.0, -1.0], // 0: back-bottom-left
    [1.0, -1.0, -1.0],  // 1: back-bottom-right
    [1.0, 1.0, -1.0],   // 2: back-top-right
    [-1.0, 1.0, -1.0],  // 3: back-top-left
    [-1.0, -1.0, 1.0],  // 4: front-bottom-left
    [1.0, -1.0, 1.0],   // 5: front-bottom-right
    [1.0, 1.0, 1.0],    // 6: front-top-right
    [-1.0, 1.0, 1.0],   // 7: front-top-left
];

const CUBE_EDGES: [(usize, usize); 12] = [
    (0, 1), (1, 2), (2, 3), (3, 0),  // back face
    (4, 5), (5, 6), (6, 7), (7, 4),  // front face
    (0, 4), (1, 5), (2, 6), (3, 7),  // connecting edges
];

pub struct AsciiCube {
    angle_x: f32,
    angle_y: f32,
    angle_z: f32,
    canvas_width: usize,
    canvas_height: usize,
    rotation_speed: f32,
}

impl AsciiCube {
    pub fn new(width: usize, height: usize, speed: f32) -> Self {
        Self {
            angle_x: 0.0,
            angle_y: 0.0,
            angle_z: 0.0,
            canvas_width: width,
            canvas_height: height,
            rotation_speed: speed,
        }
    }

    pub fn new_auto_size(speed: f32) -> Self {
        let (width, height) = Self::get_terminal_size();
        Self {
            angle_x: 0.0,
            angle_y: 0.0,
            angle_z: 0.0,
            canvas_width: width,
            canvas_height: height,
            rotation_speed: speed,
        }
    }

    fn get_terminal_size() -> (usize, usize) {
        if let Some((Width(w), Height(h))) = terminal_size() {
            // Use 80% of terminal width/height
            let width = ((w as f32 * 0.8) as usize).max(20);
            let height = ((h as f32 * 0.8) as usize).max(10);
            (width, height)
        } else {
            // Fallback size if can't detect terminal
            (40, 20)
        }
    }

    fn rotate_point(&self, point: [f32; 3]) -> [f32; 3] {
        // Rotate around X axis
        let (sin_x, cos_x) = self.angle_x.sin_cos();
        let y1 = point[1] * cos_x - point[2] * sin_x;
        let z1 = point[1] * sin_x + point[2] * cos_x;

        // Rotate around Y axis
        let (sin_y, cos_y) = self.angle_y.sin_cos();
        let x2 = point[0] * cos_y + z1 * sin_y;
        let z2 = -point[0] * sin_y + z1 * cos_y;

        // Rotate around Z axis
        let (sin_z, cos_z) = self.angle_z.sin_cos();
        let x3 = x2 * cos_z - y1 * sin_z;
        let y3 = x2 * sin_z + y1 * cos_z;

        [x3, y3, z2]
    }

    fn project_point(&self, point: [f32; 3]) -> (i32, i32) {
        // Dynamic scaling based on terminal size
        let scale = (self.canvas_width.min(self.canvas_height) as f32 * 0.3).max(10.0);
        let depth = 5.0;
        
        let z = depth / (depth + point[2]);
        let x = ((point[0] * z * scale) + self.canvas_width as f32 / 2.0) as i32;
        let y = ((point[1] * z * scale) + self.canvas_height as f32 / 2.0) as i32;
        
        (x, y)
    }

    pub fn render(&self) -> String {
        let mut canvas = vec![vec![' '; self.canvas_width]; self.canvas_height];
        
        // Project and draw edges
        for (start_idx, end_idx) in CUBE_EDGES.iter() {
            let start = self.rotate_point(CUBE_VERTICES[*start_idx]);
            let end = self.rotate_point(CUBE_VERTICES[*end_idx]);
            
            let (x1, y1) = self.project_point(start);
            let (x2, y2) = self.project_point(end);
            
            // Draw line between points using Bresenham's algorithm
            self.draw_line(&mut canvas, x1, y1, x2, y2);
        }
        
        // Convert canvas to string
        canvas.iter()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn draw_line(&self, canvas: &mut Vec<Vec<char>>, x1: i32, y1: i32, x2: i32, y2: i32) {
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = if dx > dy { dx } else { -dy } / 2;
        
        let mut x = x1;
        let mut y = y1;
        
        loop {
            if x >= 0 && x < self.canvas_width as i32 && 
               y >= 0 && y < self.canvas_height as i32 {
                // Use different characters based on position
                canvas[y as usize][x as usize] = if (x + y) % 2 == 0 { '.' } else { ',' };
            }
            
            if x == x2 && y == y2 { break; }
            
            let e2 = err;
            if e2 > -dx {
                err -= dy;
                x += sx;
            }
            if e2 < dy {
                err += dx;
                y += sy;
            }
        }
    }

    pub fn update(&mut self) {
        self.angle_x += self.rotation_speed * 0.03;
        self.angle_y += self.rotation_speed * 0.02;
        self.angle_z += self.rotation_speed * 0.01;
        
        // Keep angles between 0 and 2π
        self.angle_x %= 2.0 * PI;
        self.angle_y %= 2.0 * PI;
        self.angle_z %= 2.0 * PI;
    }

    fn update_terminal_size(&mut self) {
        let (width, height) = Self::get_terminal_size();
        self.canvas_width = width;
        self.canvas_height = height;
    }

    pub fn start_animation(&mut self) {
        let mut last_resize_check = std::time::Instant::now();
        
        loop {
            // Check terminal size every 500ms
            if last_resize_check.elapsed() >= Duration::from_millis(500) {
                self.update_terminal_size();
                last_resize_check = std::time::Instant::now();
            }

            // Clear screen (ANSI escape code)
            print!("\x1B[2J\x1B[1;1H");
            
            // Render and print cube
            println!("{}", self.render());
            
            // Update rotation
            self.update();
            
            // Control animation speed
            thread::sleep(Duration::from_millis(50));
        }
    }
}

pub fn display_rotating_cube() {
    let mut cube = AsciiCube::new_auto_size(1.0);
    println!("\nDisplaying ASCII Cube Animation (Press Ctrl+C to stop)...\n");
    cube.start_animation();
}