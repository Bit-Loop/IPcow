use std::f32::consts::PI;
use std::thread;
use std::time::Duration;
use std::io::{stdout, Write};
use std::thread::sleep;
use terminal_size::{Width, Height, terminal_size};
use nalgebra::{Matrix2, Matrix3, Vector2, Vector3, Rotation3, Const, ArrayStorage};

const CUBE_VERTICES: [[f32; 3]; 8] = [
    [-1.0, -1.0, -1.0], // 0: back-bottom-left
    [1.0, -1.0, -1.0],  // 1: back-9bottom-right
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
    // Existing fields
    angle_x: f32,
    angle_y: f32,
    angle_z: f32,
    canvas_width: usize,
    canvas_height: usize,
    rotation_speed: f32,
    lambda: f32,  // New parameter for exponential scaling
    time: f32,    // Time accumulator for smooth animation
    transformation_matrix: [[f32; 3]; 3], // Added transformation matrix
    
    // New fields for enhanced math visualization
    velocity: Vector3<f32>,
    system_matrix: Matrix3<f32>,
    phase_space: Vec<Vector2<f32>>,
    eigenvectors: Vec<Vector3<f32>>,
    eigenvalues: Vec<f32>,
    time_step: f32,
    
    // Display state
    show_eigenvectors: bool,
    show_phase_space: bool,
    show_details: bool, // New parameter to toggle details

    // New fields for scale control
    current_scale: f32,
    scale_direction: f32,

    // Add double buffer
    buffer_a: Vec<Vec<(char, &'static str)>>,
    buffer_b: Vec<Vec<(char, &'static str)>>,
    current_buffer: bool,
}

impl AsciiCube {
    // Add more color constants
    const COLORS: [&'static str; 12] = [
        "\x1b[31m", // Red
        "\x1b[33m", // Yellow
        "\x1b[32m", // Green
        "\x1b[36m", // Cyan
        "\x1b[34m", // Blue
        "\x1b[35m", // Magenta
        "\x1b[91m", // Light Red
        "\x1b[93m", // Light Yellow
        "\x1b[92m", // Light Green
        "\x1b[96m", // Light Cyan
        "\x1b[94m", // Light Blue
        "\x1b[95m", // Light Magenta
    ];

    // Add constants for scale control
    const MIN_SCALE: f32 = 0.2;
    const MAX_SCALE: f32 = 2.0;
    const SCALE_RATE: f32 = 0.01;

    // Add new constants for mathematical stability
    const EIGENVALUE_SCALE: f32 = 0.1;
    const ROTATION_DAMPING: f32 = 0.95;
    const SCALE_BOUNDS: (f32, f32) = (0.3, 1.8);

    const SMOOTHING_FACTOR: f32 = 0.1;
    const SIZE_UPDATE_THRESHOLD: f32 = 0.05;

    fn get_color(&self, point: [f32; 3], eigenvalue: f32) -> &'static str {
        // Improved color mapping with z-depth and eigenvalue influence
        let depth = ((point[2] + 1.0) * 0.5).powf(0.8); // Gamma correction
        let eigen_factor = (eigenvalue * Self::EIGENVALUE_SCALE).tanh() * 0.5 + 0.5;
        let energy = (self.calculate_energy() * 0.1).tanh();
        
        // Smooth color transition
        let color_factor = Self::lerp(
            depth * 0.4 + eigen_factor * 0.4,
            energy * 0.2,
            self.current_scale
        );
        
        let index = ((color_factor * (Self::COLORS.len() - 1) as f32) as usize)
            .clamp(0, Self::COLORS.len() - 1);
        
        Self::COLORS[index]
    }

    fn lerp(start: f32, end: f32, alpha: f32) -> f32 {
        start + (end - start) * alpha.clamp(0.0, 1.0)
    }

    fn smooth_terminal_update(&mut self) {
        let (target_width, target_height) = Self::get_terminal_size();
        
        self.canvas_width = Self::lerp(
            self.canvas_width as f32, 
            target_width as f32, 
            Self::SMOOTHING_FACTOR
        ) as usize;
        
        self.canvas_height = Self::lerp(
            self.canvas_height as f32, 
            target_height as f32, 
            Self::SMOOTHING_FACTOR
        ) as usize;
    }

    pub fn new(width: usize, height: usize, speed: f32) -> Self {
        // Initialize system matrix for coupled DEs
        let system_matrix = Matrix3::new(
            2.0, -1.0,  0.0,
            1.0,  3.0,  0.0,
            0.0,  0.0,  1.0
        );
        
        // Calculate eigenvalues and eigenvectors
        let eigen = system_matrix.symmetric_eigen();
        
        Self {
            // Existing initializations...
            angle_x: 0.0,
            angle_y: 0.0,
            angle_z: 0.0,
            canvas_width: width,
            canvas_height: height,
            rotation_speed: speed,
            lambda: 0.5,  // Exponential growth rate
            time: 0.0,
            transformation_matrix: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            
            // New initializations
            velocity: Vector3::zeros(),
            system_matrix,
            phase_space: Vec::new(),
            eigenvectors: eigen.eigenvectors.column_iter().map(|c| c.into_owned().into()).collect(),
            eigenvalues: eigen.eigenvalues.data.0[0].iter().copied().collect(),
            time_step: 0.05,
            show_eigenvectors: true,
            show_phase_space: true,
            show_details: false, // Default to not showing details

            // New initializations for scale control
            current_scale: 1.0,
            scale_direction: -1.0, // Start shrinking

            // Add double buffer
            buffer_a: vec![vec![(' ', "\x1b[0m"); width]; height],
            buffer_b: vec![vec![(' ', "\x1b[0m"); width]; height],
            current_buffer: false,
        }
    }

    pub fn new_auto_size(speed: f32) -> Self {
        let (width, height) = Self::get_terminal_size();
        let empty_cell = (' ', "\x1b[0m");
        let buffer_a = vec![vec![empty_cell; width]; height];
        let buffer_b = vec![vec![empty_cell; width]; height];
        
        // Generate random eigenvalues for more interesting behavior
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let system_matrix = Matrix3::new(
            rng.gen_range(-2.0..2.0), rng.gen_range(-1.0..1.0), 0.0,
            rng.gen_range(-1.0..1.0), rng.gen_range(-2.0..2.0), 0.0,
            0.0, 0.0, rng.gen_range(0.5..1.5)
        );
        
        let eigen = system_matrix.symmetric_eigen();
        
        Self {
            angle_x: 0.0,
            angle_y: 0.0,
            angle_z: 0.0,
            canvas_width: width,
            canvas_height: height,
            rotation_speed: speed,
            lambda: 0.3, // Reduced initial lambda
            time: 0.0,
            transformation_matrix: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            velocity: Vector3::zeros(),
            system_matrix,
            phase_space: Vec::new(),
            eigenvectors: eigen.eigenvectors.column_iter().map(|c| c.into()).collect(),
            eigenvalues: eigen.eigenvalues.data.0[0].iter().copied().collect(),
            time_step: 0.05,
            show_eigenvectors: true,
            show_phase_space: true,
            show_details: false, // Default to not showing details

            // New initializations for scale control
            current_scale: 1.0,
            scale_direction: -1.0, // Start shrinking

            // Add double buffer
            buffer_a,
            buffer_b,
            current_buffer: false,
        }
    }

    fn get_terminal_size() -> (usize, usize) {
        if let Some((Width(w), Height(h))) = terminal_size() {
            // Use 80% of terminal width/height
            let width = ((w as f32 * 0.8) as usize).max(30).min(3840);
            let height = ((h as f32 * 0.8) as usize).max(30).min(2160);
            (width, height)
        } else {
            // Fallback size if can't detect terminal
            (40, 20)
        }
    }

    fn calculate_eigenvalue_transformation(&mut self) {
        let lambda = self.lambda * self.current_scale;
        let t = self.time;
        
        // Stable eigenvalue calculation
        let eigenvalue = (lambda * t).tanh(); // Use tanh for bounded growth
        
        self.transformation_matrix = [
            [eigenvalue.cos(), -eigenvalue.sin(), 0.0],
            [eigenvalue.sin(), eigenvalue.cos(), 0.0],
            [0.0, 0.0, 1.0],
        ];
    }

    fn apply_transformation(&self, point: [f32; 3]) -> [f32; 3] {
        // Apply linear transformation using matrix multiplication
        [
            point[0] * self.transformation_matrix[0][0] + 
            point[1] * self.transformation_matrix[0][1] + 
            point[2] * self.transformation_matrix[0][2],
            
            point[0] * self.transformation_matrix[1][0] + 
            point[1] * self.transformation_matrix[1][1] + 
            point[2] * self.transformation_matrix[1][2],
            
            point[0] * self.transformation_matrix[2][0] + 
            point[1] * self.transformation_matrix[2][1] + 
            point[2] * self.transformation_matrix[2][2],
        ]
    }

    fn rotate_point(&self, point: [f32; 3]) -> [f32; 3] {
        // Get eigenvalue influence
        let eigen_scale = self.eigenvalues[0].tanh() * 0.5 + 0.5;
        
        // Original rotation code with eigenvalue scaling
        let (sin_x, cos_x) = (self.angle_x * eigen_scale).sin_cos();
        let y1 = point[1] * cos_x - point[2] * sin_x;
        let z1 = point[1] * sin_x + point[2] * cos_x;

        let (sin_y, cos_y) = (self.angle_y * eigen_scale).sin_cos();
        let x2 = point[0] * cos_y + z1 * sin_y;
        let z2 = -point[0] * sin_y + z1 * cos_y;

        let (sin_z, cos_z) = (self.angle_z * eigen_scale).sin_cos();
        let x3 = x2 * cos_z - y1 * sin_z;
        let y3 = x2 * sin_z + y1 * cos_z;

        [x3, y3, z2]
    }

    fn project_point(&self, point: &[f32]) -> (i32, i32) {
        let scale = (self.canvas_width.min(self.canvas_height) as f32 * 0.3).max(10.0);
        let adjusted_scale = scale * self.current_scale;
        
        let depth = 5.0;
        let z = depth / (depth + point[2]);
        
        let x = ((point[0] * z * adjusted_scale) + self.canvas_width as f32 / 2.0) as i32;
        let y = ((point[1] * z * adjusted_scale) + self.canvas_height as f32 / 2.0) as i32;
        
        (x, y)
    }

    pub fn render(&mut self) -> String {
        let buffer = self.render_buffer();
        self.buffer_to_string(&buffer)
    }

    fn buffer_to_string(&self, buffer: &Vec<Vec<(char, &'static str)>>) -> String {
        buffer.iter()
            .map(|row| {
                row.iter()
                    .map(|(c, color)| format!("{}{}", color, c))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn draw_line(canvas: &mut Vec<Vec<(char, &'static str)>>, x1: i32, y1: i32, x2: i32, y2: i32, z_depth: f32, colors: &[&'static str], eigenvalues: &[f32], canvas_width: usize, canvas_height: usize) {
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = if dx > dy { dx } else { -dy } / 2;

        let mut x = x1;
        let mut y = y1;
        
        // Use z-depth and eigenvalues for color
        let eigen_factor = (eigenvalues[0] * z_depth).tanh();
        let color_index = ((eigen_factor + 1.0) * (colors.len() - 1) as f32 / 2.0) as usize;
        let color = colors[color_index.clamp(0, colors.len() - 1)];

        loop {
            if x >= 0 && x < canvas_width as i32 && y >= 0 && y < canvas_height as i32 {
                canvas[y as usize][x as usize] = ('.', color);
            }
            
            if x == x2 && y == y2 { break; }
            
            let e2 = err;
            if e2 > -dx { err -= dy; x += sx; }
            if e2 < dy { err += dx; y += sy; }
        }
    }

    fn calculate_transformation_matrix(&mut self) {
        // Create scale matrix
        let scale = Matrix3::new_scaling(self.current_scale);
        
        // Create rotation matrices using correct axis scaling
        let rot_x = Rotation3::from_axis_angle(&Vector3::x_axis(), self.angle_x);
        let rot_y = Rotation3::from_axis_angle(&Vector3::y_axis(), self.angle_y);
        let rot_z = Rotation3::from_axis_angle(&Vector3::z_axis(), self.angle_z);
        
        // Combine transformations and convert to array
        let result = scale * rot_z * rot_y * rot_x;
        self.transformation_matrix = [
            [result[(0, 0)], result[(0, 1)], result[(0, 2)]],
            [result[(1, 0)], result[(1, 1)], result[(1, 2)]],
            [result[(2, 0)], result[(2, 1)], result[(2, 2)]],
        ];
    }

    pub fn update(&mut self) {
        // Smooth rotation with eigenvalue influence
        let eigen_dampening = self.eigenvalues.iter().map(|e| e.tanh()).sum::<f32>() / 3.0;
        
        self.angle_x += self.rotation_speed * 0.03 * eigen_dampening;
        self.angle_y += self.rotation_speed * 0.02 * eigen_dampening;
        self.angle_z += self.rotation_speed * 0.01 * eigen_dampening;
        
        // Keep angles bounded
        self.angle_x %= 2.0 * PI;
        self.angle_y %= 2.0 * PI;
        self.angle_z %= 2.0 * PI;

        // Update system matrix state
        self.update_phase_space();
    }

    fn update_phase_space(&mut self) {
        // Update system state using the coupled DEs
        let state = Vector3::new(self.angle_x, self.angle_y, self.angle_z);
        let derivative = self.system_matrix * state;
        
        // Store phase space trajectory
        self.phase_space.push(Vector2::new(state[0], state[1]));
        if self.phase_space.len() > 100 {
            self.phase_space.remove(0);
        }
        
        // Update velocities using eigenvalue-based scaling
        self.velocity = derivative.component_mul(&Vector3::new(
            self.eigenvalues[0].exp(),
            self.eigenvalues[1].exp(),
            self.eigenvalues[2].exp(),
        ));
    }

    fn calculate_energy(&self) -> f32 {
        self.velocity.norm_squared() / 2.0
    }

    // Demonstrate a simple 2D eigenvalue system alongside the 3D cube
    pub fn test_eigensystem(&self) {
        let a = Matrix2::new(2.0, -1.0, 1.0, 3.0);
        let eigen = a.symmetric_eigen();
        println!("Eigenvalues: {:?}", eigen.eigenvalues);
        println!("Eigenvectors:\n{:?}", eigen.eigenvectors);

        let mut state = Vector2::new(1.0, 0.0);
        let dt = 0.1;
        let total_time = 2.0; // shorter for quick demo

        println!("Time evolution of the system:");
        for t in (0..(total_time / dt) as usize).map(|x| x as f32 * dt) {
            state = a * state * dt;
            println!("t = {:.2}: x = {:.4}, y = {:.4}", t, state[0], state[1]);
        }
    }

    pub fn start_animation(&mut self) {
        // Set up ctrl+c handler for cleanup
        ctrlc::set_handler(|| {
            print!("\x1B[?25h"); // Show cursor
            print!("\x1B[2J\x1B[1;1H"); // Clear screen
            std::process::exit(0);
        }).expect("Failed to set Ctrl+C handler");

        // Hide cursor during animation
        print!("\x1B[?25l");
        
        let frame_time = Duration::from_millis(33);
        let mut last_frame = std::time::Instant::now();
        
        loop {
            self.smooth_terminal_update(); // Add dynamic terminal size handling
            let now = std::time::Instant::now();
            let elapsed = now - last_frame;
            
            if elapsed >= frame_time {
                self.update();
                print!("\x1B[2J\x1B[1;1H");
                
                // Render directly to stdout without String allocation
                let buffer = self.render_cube();
                for row in buffer {
                    for (c, color) in row {
                        print!("{}{}", color, c);
                    }
                    println!("\x1b[0m");
                }
                
                stdout().flush().unwrap();
                last_frame = now;
            } else {
                thread::sleep(frame_time - elapsed);
            }
        }
    }
}

impl AsciiCube {
    fn render_cube(&mut self) -> &Vec<Vec<(char, &'static str)>> {
        // Calculate all transformations first
        let transform = self.calculate_stable_transformation();
        let transformed_points: Vec<_> = CUBE_VERTICES.iter()
            .map(|v| transform * Vector3::from_column_slice(v))
            .collect();

        let edges: Vec<_> = CUBE_EDGES.iter()
            .map(|(start_idx, end_idx)| {
                let start = &transformed_points[*start_idx];
                let end = &transformed_points[*end_idx];
                
                let (x1, y1) = self.project_point(&[start[0], start[1], start[2]]);
                let (x2, y2) = self.project_point(&[end[0], end[1], end[2]]);
                
                ((x1, y1), (x2, y2), start[2])
            })
            .collect();

        let buffer = if self.current_buffer {
            &mut self.buffer_a
        } else {
            &mut self.buffer_b
        };

        buffer.iter_mut().for_each(|row| {
            row.fill((' ', "\x1b[0m"));
        });

        // Draw all edges
        for ((x1, y1), (x2, y2), z_depth) in edges {
            Self::draw_line(buffer, x1, y1, x2, y2, z_depth, &Self::COLORS, &self.eigenvalues, self.canvas_width, self.canvas_height);
        }

        self.current_buffer = !self.current_buffer;
        buffer
    }

    fn calculate_stable_transformation(&self) -> Matrix3<f32> {
        // Create basic transformations
        let scale = Matrix3::new_scaling(self.current_scale);
        
        // Create rotation matrices using angles directly
        let rot_x = Rotation3::from_axis_angle(&Vector3::x_axis(), self.angle_x).to_homogeneous().fixed_resize::<3, 3>(0.0);
        let rot_y = Rotation3::from_axis_angle(&Vector3::y_axis(), self.angle_y).to_homogeneous().fixed_resize::<3, 3>(0.0);
        let rot_z = Rotation3::from_axis_angle(&Vector3::z_axis(), self.angle_z).to_homogeneous().fixed_resize::<3, 3>(0.0);
        
        // Combine transformations in correct order
        scale * (rot_z * rot_y * rot_x)
    }

    fn render_buffer(&mut self) -> Vec<Vec<(char, &'static str)>> {
        let mut buffer = vec![vec![(' ', "\x1b[0m"); self.canvas_width]; self.canvas_height];
        let transform = self.calculate_stable_transformation();
        
        // Transform vertices using fixed array construction
        let transformed_points: Vec<Vector3<f32>> = CUBE_VERTICES.iter()
            .map(|v| transform * Vector3::from_column_slice(v))
            .collect();
        
        // Rest of the rendering code...
        for &(start_idx, end_idx) in CUBE_EDGES.iter() {
            let start = &transformed_points[start_idx];
            let end = &transformed_points[end_idx];
            
            let (x1, y1) = self.project_point(&[start[0], start[1], start[2]]);
            let (x2, y2) = self.project_point(&[end[0], end[1], end[2]]);
            
            AsciiCube::draw_line(&mut buffer, x1, y1, x2, y2, start[2], &Self::COLORS, &self.eigenvalues, self.canvas_width, self.canvas_height);
        }
        
        buffer
    }
}

pub fn display_rotating_cube() {
    let mut cube = AsciiCube::new_auto_size(1.0);
    println!("\nDisplaying ASCII Cube Animation (Press Ctrl+C to stop)...\n");
    cube.start_animation();
}