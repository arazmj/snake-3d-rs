use three_d::*;
use crate::game::{GameState, Position, Face};

pub struct GameRenderer {
    context: Context,
    camera: Camera,
    control: OrbitControl,
    board_instances: Gm<InstancedMesh, PhysicalMaterial>,
    snake_instances: Gm<InstancedMesh, PhysicalMaterial>,
    food_mesh: Gm<Mesh, PhysicalMaterial>,
    grid_size: i32,
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
        let control = OrbitControl::new(*camera.target(), 1.0, 100.0);

        // Board Voxels
        let mut board_transformations = Vec::new();
        let cell_size = 2.0 / grid_size as f32;
        let voxel_scale = cell_size * 0.95; // Slightly smaller for gaps

        for x in 0..grid_size {
            for y in 0..grid_size {
                for z in 0..grid_size {
                    // We only want the surface voxels? 
                    // The user image shows a solid-looking block of voxels, or at least the outer shell.
                    // If we render the inside, it might look too dense with transparency.
                    // But "The Board: A large Cube divided into a 10x10 grid" implies the whole volume or surface.
                    // The reference image looks like a solid block of glass cubes.
                    // Let's render all of them for the full effect, or just the shell if performance/visuals demand.
                    // 10x10x10 is 1000 instances, which is trivial for InstancedMesh.
                    
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

        // board_instances was created above but we need to assign it to the struct later.
        // Wait, I created it twice in the previous edit.
        // One with default material (unused) and one with custom material.
        // I need to remove the first one.
        // Enable transparency
        // Note: For correct transparency of many overlapping objects, we might need depth sorting or specific blend modes.
        // three-d does some sorting.
        // Let's set the blend mode.
        // We need to access the material inside the Gm.
        // Actually, we can set it on the material before creating Gm if we kept it mutable, 
        // or just rely on `new` handling alpha < 255.
        // `PhysicalMaterial::new` detects alpha and sets transparent render state usually.
        // But let's ensure it.
        // We can't easily modify the material inside Gm without destructuring or using mutable access if available.
        // Let's construct material first.
        
        // Re-doing construction to set render states
        let mut board_material = PhysicalMaterial::new(
            &context,
            &CpuMaterial {
                albedo: Srgba::new(0, 0, 255, 30), // Very transparent blue
                roughness: 0.2,
                metallic: 0.8, // Shiny
                ..Default::default()
            },
        );
        board_material.render_states.blend = Blend::TRANSPARENCY;
        // board_material.render_states.write_mask = WriteMask::COLOR; // Don't write depth for transparent things to avoid occlusion artifacts? 
        // If we don't write depth, back faces will show through front faces regardless of order, which is good for "glass block".
        board_material.render_states.write_mask = WriteMask::COLOR;

        let board_instances = Gm::new(
            InstancedMesh::new(&context, &Instances {
                transformations: board_transformations, 
                ..Default::default()
            }, &CpuMesh::cube()),
            board_material,
        );
        
        // Snake Instances
        let snake_instances = Gm::new(
            InstancedMesh::new(&context, &Instances::default(), &CpuMesh::cube()), 
            PhysicalMaterial::new(
                &context,
                &CpuMaterial {
                    albedo: Srgba::new_opaque(50, 200, 50), // Green snake
                    roughness: 0.3,
                    ..Default::default()
                },
            ),
        );

        // Food Mesh
        let food_mesh = Gm::new(
            Mesh::new(&context, &CpuMesh::cube()), // Cube for food
            PhysicalMaterial::new(
                &context,
                &CpuMaterial {
                    albedo: Srgba::new_opaque(200, 50, 50), // Red food
                    emissive: Srgba::new_opaque(100, 0, 0),
                    ..Default::default()
                },
            ),
        );

        Self {
            context,
            camera,
            control,
            board_instances,
            snake_instances,
            food_mesh,
            grid_size,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.camera.set_viewport(Viewport::new_at_origo(width, height));
    }

    pub fn update_camera(&mut self, events: &mut [Event]) {
        self.control.handle_events(&mut self.camera, events);
    }

    pub fn render(&mut self, game: &GameState, target: &RenderTarget) {
        let cell_size = 2.0 / self.grid_size as f32;
        let offset = 0.05; // Lift off surface

        // Update Snake Instances
        let transformations: Vec<Mat4> = game.snake.body.iter().map(|pos| {
            let center = self.pos_to_vec3(*pos, cell_size, offset);
            Mat4::from_translation(center) * Mat4::from_scale(cell_size * 0.9) // Slightly smaller than cell
        }).collect();
        
        let instances = Instances {
            transformations,
            ..Default::default()
        };
        self.snake_instances.geometry.set_instances(&instances);

        // Update Food Position
        let food_pos = self.pos_to_vec3(game.food, cell_size, offset);
        self.food_mesh.set_transformation(
            Mat4::from_translation(food_pos) * Mat4::from_scale(cell_size * 0.4) // Small sphere
        );

        // Render
        let ambient = AmbientLight::new(&self.context, 0.4, Srgba::WHITE);
        let directional = DirectionalLight::new(&self.context, 2.0, Srgba::WHITE, &vec3(1.0, 1.0, 1.0));
        let lights: &[&dyn Light] = &[&ambient, &directional];

        // Clear
        target.clear(ClearState::color_and_depth(0.1, 0.1, 0.1, 1.0, 1.0)); // Dark grey

        // Render objects
        let objects: &[&dyn Object] = &[&self.board_instances, &self.snake_instances, &self.food_mesh];
        target.render(&self.camera, objects, lights);
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
