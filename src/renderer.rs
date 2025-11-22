use three_d::*;
use crate::game::{GameState, Position, Face};

pub struct GameRenderer {
    context: Context,
    camera: Camera,
    // control: OrbitControl, // Disabled for auto-camera
    board_instances: Gm<InstancedMesh, PhysicalMaterial>,
    grid_instances: Gm<InstancedMesh, PhysicalMaterial>,
    snake_instances: Gm<InstancedMesh, PhysicalMaterial>,
    food_mesh: Gm<Mesh, PhysicalMaterial>,
    prize_mesh: Gm<Mesh, PhysicalMaterial>,
    particle_system: Gm<InstancedMesh, PhysicalMaterial>,
    particles: Vec<Particle>,
    grid_size: i32,
    target_pos: Vec3,
    target_up: Vec3,
    time: f64,
}

struct Particle {
    start_pos: Vec3,
    velocity: Vec3,
    spawn_time: f64,
    color: Srgba,
}

impl GameRenderer {
    pub fn new(context: Context, grid_size: i32) -> Self {
        let camera = Camera::new_perspective(
            Viewport::new_at_origo(1, 1),
            vec3(4.0, 4.0, 4.0),
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            degrees(45.0),
            0.1,
            100.0,
        );
        // let control = OrbitControl::new(*camera.target(), 1.0, 100.0);

        // Board Voxels
        let mut board_transformations = Vec::new();
        let cell_size = 2.0 / grid_size as f32;
        let voxel_scale = cell_size * 0.95; // Slightly smaller for gaps

        for x in 0..grid_size {
            for y in 0..grid_size {
                for z in 0..grid_size {
                    // Position
                    // Map 0..N to -1..1
                    // Center of voxel i is -1 + (i * cell_size) + cell_size/2
                    let cx = -1.0 + (x as f32 * cell_size) + cell_size / 2.0;
                    let cy = -1.0 + (y as f32 * cell_size) + cell_size / 2.0;
                    let cz = -1.0 + (z as f32 * cell_size) + cell_size / 2.0;
                    
                    board_transformations.push(
                        Mat4::from_translation(vec3(cx, cy, cz)) * Mat4::from_scale(voxel_scale)
                    );
                }
            }
        }

        let mut board_material = PhysicalMaterial::new(
            &context,
            &CpuMaterial {
                albedo: Srgba::new(50, 50, 200, 50), // Transparent blueish
                roughness: 0.1,
                metallic: 0.9,
                emissive: Srgba::new(0, 0, 50, 255), // Slight glow
                ..Default::default()
            },
        );
        board_material.render_states.blend = Blend::TRANSPARENCY;
        board_material.render_states.write_mask = WriteMask::COLOR;

        // Use spheres for rounder look
        let board_instances = Gm::new(
            InstancedMesh::new(&context, &Instances {
                transformations: board_transformations, 
                ..Default::default()
            }, &CpuMesh::sphere(16)),
            board_material,
        );

        // Grid Lines (3D Beams)
        let mut grid_transformations = Vec::new();
        let step = 2.0 / grid_size as f32;
        let offset = 0.002; // Slightly above surface
        let thickness = 0.02; // Thickness of the grid lines

        // Helper to add beam
        let mut add_beam = |pos: Vec3, scale: Vec3| {
            grid_transformations.push(
                Mat4::from_translation(pos) * Mat4::from_nonuniform_scale(scale.x, scale.y, scale.z)
            );
        };

        // Generate grid for each face
        for i in 0..=grid_size {
            let t = -1.0 + (i as f32 * step);
            
            // Front & Back (z = +/- 1)
            // Vertical lines
            add_beam(vec3(t, 0.0, 1.0 + offset), vec3(thickness, 1.0, thickness)); // Front
            add_beam(vec3(t, 0.0, -1.0 - offset), vec3(thickness, 1.0, thickness)); // Back
            // Horizontal lines
            add_beam(vec3(0.0, t, 1.0 + offset), vec3(1.0, thickness, thickness)); // Front
            add_beam(vec3(0.0, t, -1.0 - offset), vec3(1.0, thickness, thickness)); // Back

            // Left & Right (x = +/- 1)
            // Vertical lines (y axis)
            add_beam(vec3(1.0 + offset, 0.0, t), vec3(thickness, 1.0, thickness)); // Right
            add_beam(vec3(-1.0 - offset, 0.0, t), vec3(thickness, 1.0, thickness)); // Left
            // Horizontal lines (z axis)
            add_beam(vec3(1.0 + offset, t, 0.0), vec3(thickness, thickness, 1.0)); // Right
            add_beam(vec3(-1.0 - offset, t, 0.0), vec3(thickness, thickness, 1.0)); // Left

            // Top & Bottom (y = +/- 1)
            // Lines along x
            add_beam(vec3(0.0, 1.0 + offset, t), vec3(1.0, thickness, thickness)); // Top
            add_beam(vec3(0.0, -1.0 - offset, t), vec3(1.0, thickness, thickness)); // Bottom
            // Lines along z
            add_beam(vec3(t, 1.0 + offset, 0.0), vec3(thickness, thickness, 1.0)); // Top
            add_beam(vec3(t, -1.0 - offset, 0.0), vec3(thickness, thickness, 1.0)); // Bottom
        }

        let grid_instances = Gm::new(
            InstancedMesh::new(&context, &Instances {
                transformations: grid_transformations,
                ..Default::default()
            }, &CpuMesh::cube()),
            PhysicalMaterial::new(
                &context,
                &CpuMaterial {
                    albedo: Srgba::new(0, 255, 255, 255), // Bright Cyan
                    emissive: Srgba::new(0, 200, 200, 255), // Glowing
                    roughness: 0.5,
                    metallic: 0.5,
                    ..Default::default()
                },
            ),
        );
        
        // Snake Instances
        let snake_instances = Gm::new(
            InstancedMesh::new(&context, &Instances::default(), &CpuMesh::sphere(16)),
            PhysicalMaterial::new(
                &context,
                &CpuMaterial {
                    albedo: Srgba::new_opaque(50, 200, 50), // Green snake
                    emissive: Srgba::new_opaque(50, 150, 50), // Glow
                    roughness: 0.2,
                    metallic: 0.5,
                    ..Default::default()
                },
            ),
        );

        // Food Mesh - Sphere
        let food_mesh = Gm::new(
            Mesh::new(&context, &CpuMesh::sphere(16)),
            PhysicalMaterial::new(
                &context,
                &CpuMaterial {
                    albedo: Srgba::new_opaque(200, 50, 50), // Red food
                    emissive: Srgba::new_opaque(100, 0, 0),
                    ..Default::default()
                },
            ),
        );

        // Prize Mesh - Cylinder (Gold)
        let prize_mesh = Gm::new(
            Mesh::new(&context, &CpuMesh::cylinder(16)),
            PhysicalMaterial::new(
                &context,
                &CpuMaterial {
                    albedo: Srgba::new_opaque(255, 215, 0), // Gold
                    emissive: Srgba::new_opaque(100, 80, 0),
                    roughness: 0.1,
                    metallic: 0.9,
                    ..Default::default()
                },
            ),
        );

        // Particle System
        let particle_system = Gm::new(
            InstancedMesh::new(&context, &Instances::default(), &CpuMesh::sphere(8)),
            PhysicalMaterial::new(
                &context,
                &CpuMaterial {
                    albedo: Srgba::WHITE,
                    emissive: Srgba::WHITE,
                    ..Default::default()
                }
            )
        );

        Self {
            context,
            camera,
            // control,
            board_instances,
            grid_instances,
            snake_instances,
            food_mesh,
            prize_mesh,
            particle_system,
            particles: Vec::new(),
            grid_size,
            target_pos: vec3(0.0, 0.0, 4.0),
            target_up: vec3(0.0, 1.0, 0.0),
            time: 0.0,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.camera.set_viewport(Viewport::new_at_origo(width, height));
    }

    pub fn update_camera(&mut self, _events: &mut [Event]) {
        // self.control.handle_events(&mut self.camera, events);
    }

    pub fn update_camera_target(&mut self, face: Face, distance: f32) {
        let (pos, up) = match face {
            Face::Front => (vec3(0.0, 0.0, distance), vec3(0.0, 1.0, 0.0)),
            Face::Back => (vec3(0.0, 0.0, -distance), vec3(0.0, 1.0, 0.0)),
            Face::Left => (vec3(-distance, 0.0, 0.0), vec3(0.0, 1.0, 0.0)),
            Face::Right => (vec3(distance, 0.0, 0.0), vec3(0.0, 1.0, 0.0)),
            Face::Top => (vec3(0.0, distance, 0.0), vec3(0.0, 0.0, -1.0)),
            Face::Bottom => (vec3(0.0, -distance, 0.0), vec3(0.0, 0.0, 1.0)),
        };
        
        self.target_pos = pos;
        self.target_up = up;
    }

    pub fn render(&mut self, game: &GameState, target: &RenderTarget, dt: f64) {
        self.time += dt;

        // Calculate required distance based on aspect ratio
        let viewport = self.camera.viewport();
        let aspect = viewport.width as f32 / viewport.height as f32;
        let base_dist = 4.5; // Increased slightly from 4.0 for better padding
        let dist = if aspect < 1.0 {
            base_dist / aspect
        } else {
            base_dist
        };

        // Update Camera Position based on Snake Head
        self.update_camera_target(game.snake.head().face, dist);

        // Smoothly interpolate camera
        let speed = 5.0; // Adjust for smoothness
        let t = (speed * dt as f32).min(1.0);
        
        let current_pos = *self.camera.position();
        let current_up = *self.camera.up();
        
        let new_pos = current_pos.lerp(self.target_pos, t);
        let new_up = current_up.lerp(self.target_up, t).normalize();
        
        self.camera = Camera::new_perspective(
            self.camera.viewport(),
            new_pos,
            vec3(0.0, 0.0, 0.0),
            new_up,
            degrees(45.0),
            0.1,
            100.0,
        );

        let cell_size = 2.0 / self.grid_size as f32;
        let offset = 0.05; // Lift off surface

        // Detect eat event for particles
        // We need a way to detect eat from here, or pass it in.
        // Current architecture updates game logic then renders.
        // We can check if snake grew, or just rely on particle spawning being called from outside.
        // But since we don't have a method to call "spawn_particles" from lib.rs easily without exposing renderer internals there,
        // let's do it in update loop in lib.rs?
        // No, let's check if food changed position? No, food respawns.
        // Let's just spawn particles in `lib.rs` by calling a new method on renderer.

        // Update Snake Instances
        let transformations: Vec<Mat4> = game.snake.body.iter().map(|pos| {
            let center = self.pos_to_vec3(*pos, cell_size, offset);
            Mat4::from_translation(center) * Mat4::from_scale(cell_size * 0.6) // Smaller snake
        }).collect();
        
        let instances = Instances {
            transformations,
            ..Default::default()
        };
        self.snake_instances.geometry.set_instances(&instances);

        // Update Food Position & Animation
        let food_pos = self.pos_to_vec3(game.food, cell_size, offset);
        let bounce = (self.time * 5.0).sin() as f32 * 0.05;
        let rotate = Mat4::from_angle_y(radians((self.time * 2.0) as f32));

        let food_scale = if game.is_prize { cell_size * 0.5 } else { cell_size * 0.4 };
        let food_transform = Mat4::from_translation(food_pos + vec3(0.0, 0.0, bounce)) * rotate * Mat4::from_scale(food_scale);

        if game.is_prize {
            self.prize_mesh.set_transformation(food_transform);
        } else {
            self.food_mesh.set_transformation(food_transform);
        }

        // Update Particles
        let mut particle_transformations = Vec::new();
        let mut particle_colors = Vec::new();

        self.particles.retain(|p| self.time - p.spawn_time < 1.0);

        for p in &self.particles {
            let age = (self.time - p.spawn_time) as f32;
            let pos = p.start_pos + p.velocity * age;
            let scale = (1.0 - age) * 0.05;
            particle_transformations.push(Mat4::from_translation(pos) * Mat4::from_scale(scale));
            particle_colors.push(p.color);
        }

        let particle_instances = Instances {
            transformations: particle_transformations,
            colors: Some(particle_colors),
            ..Default::default()
        };
        self.particle_system.geometry.set_instances(&particle_instances);

        // Render
        let ambient = AmbientLight::new(&self.context, 0.3, Srgba::WHITE);
        let directional = DirectionalLight::new(&self.context, 2.0, Srgba::WHITE, &vec3(1.0, 1.0, 1.0));

        let point_light_color = if game.is_prize { Srgba::new_opaque(255, 215, 0) } else { Srgba::new_opaque(255, 50, 50) };
        let point_light = PointLight::new(
            &self.context,
            5.0,
            point_light_color,
            &food_pos,
            Attenuation { constant: 0.1, linear: 0.3, quadratic: 0.3 },
        );

        let lights: Vec<&dyn Light> = vec![&ambient, &directional, &point_light];
        let lights = lights.as_slice();

        // Clear
        target.clear(ClearState::color_and_depth(0.1, 0.1, 0.1, 1.0, 1.0)); // Dark grey

        // Render objects
        let mut objects: Vec<&dyn Object> = vec![&self.board_instances, &self.grid_instances, &self.snake_instances, &self.particle_system];
        if game.is_prize {
            objects.push(&self.prize_mesh);
        } else {
            objects.push(&self.food_mesh);
        }

        target.render(&self.camera, objects.as_slice(), lights);
    }

    pub fn spawn_particles(&mut self, pos: Position, is_prize: bool) {
        let cell_size = 2.0 / self.grid_size as f32;
        let offset = 0.05;
        let center = self.pos_to_vec3(pos, cell_size, offset);

        let color = if is_prize { Srgba::new_opaque(255, 215, 0) } else { Srgba::new_opaque(200, 50, 50) };

        for _ in 0..10 {
             // Simple random velocity
             let mut rng_buf = [0u8; 3];
             getrandom::getrandom(&mut rng_buf).unwrap_or(());
             let rx = (rng_buf[0] as f32 / 255.0) - 0.5;
             let ry = (rng_buf[1] as f32 / 255.0) - 0.5;
             let rz = (rng_buf[2] as f32 / 255.0) - 0.5;
             let velocity = vec3(rx, ry, rz).normalize() * 1.0; // Explosion speed

             self.particles.push(Particle {
                 start_pos: center,
                 velocity,
                 spawn_time: self.time,
                 color,
             });
        }
    }

    fn pos_to_vec3(&self, pos: Position, cell_size: f32, offset: f32) -> Vec3 {
        let u = pos.u as f32;
        let v = pos.v as f32;
        let half_size = cell_size / 2.0;
        
        // Base coordinates on face (from -1 to 1)
        // u maps to a range. 
        // 0 -> -1 + half_size
        // N-1 -> 1 - half_size
        
        let u_local = -1.0 + (u * cell_size) + half_size;
        let v_local = -1.0 + (v * cell_size) + half_size;
        
        // Surface level is 1.0 + offset (or -1.0 - offset)
        let surface = 1.0 + offset;

        match pos.face {
            Face::Front => vec3(u_local, v_local, surface),
            Face::Back => vec3(-u_local, v_local, -surface), // Note -u_local to match Right/Left logic
            Face::Right => vec3(surface, v_local, -u_local),
            Face::Left => vec3(-surface, v_local, u_local),
            Face::Top => vec3(u_local, surface, -v_local),
            Face::Bottom => vec3(u_local, -surface, v_local),
        }
    }
}
