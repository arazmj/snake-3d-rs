use wasm_bindgen::prelude::*;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub face: Face,
    pub u: i32,
    pub v: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameConfig {
    pub grid_size: i32,
}

pub struct Snake {
    pub body: VecDeque<Position>,
    pub direction: Direction,
    pub next_direction: Direction,
}

impl Snake {
    pub fn new(start_pos: Position, start_dir: Direction) -> Self {
        let mut body = VecDeque::new();
        body.push_back(start_pos);
        // Add a couple more segments for initial length
        // For simplicity, just one for now, or handle in Game::new
        Self {
            body,
            direction: start_dir,
            next_direction: start_dir,
        }
    }

    pub fn head(&self) -> Position {
        *self.body.front().unwrap()
    }
}

#[derive(PartialEq)]
pub enum GameEvent {
    None,
    Eat,
    EatPrize,
    GameOver,
}

pub struct GameState {
    pub snake: Snake,
    pub food: Position,
    pub is_prize: bool,
    pub score: u32,
    pub high_score: u32,
    pub food_eaten_count: u32,
    pub game_over: bool,
    pub config: GameConfig,
}

impl GameState {
    pub fn new(grid_size: i32) -> Self {
        let start_pos = Position {
            face: Face::Front,
            u: grid_size / 2,
            v: grid_size / 2,
        };
        let snake = Snake::new(start_pos, Direction::Up);
        // Note: High score persistence would normally be loaded from localStorage here,
        // but accessing window/localStorage in pure logic struct is messy.
        // We'll handle it in lib.rs or pass it in.
        // For now, start at 0, and update_ui will handle display if we store it externally.

        let mut game = Self {
            snake,
            food: start_pos, // Placeholder
            is_prize: false,
            score: 0,
            high_score: 0,
            food_eaten_count: 0,
            game_over: false,
            config: GameConfig { grid_size },
        };
        game.spawn_food();
        game
    }

    pub fn spawn_food(&mut self) {
        // Simple random spawn logic
        // In a real game, ensure it doesn't spawn on snake
        // Using a simple LCG or similar for determinism if needed, 
        // but for now we'll rely on `getrandom` via a helper or just passed in entropy.
        // Since we need `getrandom` which is available in WASM:
        
        let mut rng_buf = [0u8; 3];
        getrandom::getrandom(&mut rng_buf).unwrap_or(());
        
        // Map bytes to face and UV
        let face_idx = rng_buf[0] % 6;
        let face = match face_idx {
            0 => Face::Front,
            1 => Face::Back,
            2 => Face::Left,
            3 => Face::Right,
            4 => Face::Top,
            _ => Face::Bottom,
        };
        let u = (rng_buf[1] as i32) % self.config.grid_size;
        let v = (rng_buf[2] as i32) % self.config.grid_size;
        
        let new_pos = Position { face, u, v };
        
        // Check collision with snake
        if self.snake.body.contains(&new_pos) {
            self.spawn_food(); // Retry (recursive, but low probability of stack overflow for small snake)
        } else {
            self.food = new_pos;
            // Spawn a prize every 5 items
            self.is_prize = (self.food_eaten_count + 1) % 5 == 0;
        }
    }

    pub fn update(&mut self) -> GameEvent {
        if self.game_over {
            return GameEvent::None;
        }

        self.snake.direction = self.snake.next_direction;
        let head = self.snake.head();
        let (new_pos, new_dir) = self.calculate_next_position(head, self.snake.direction);

        // Check self collision
        // Note: Tail will move, so we shouldn't collide with tail unless length 2 reverses (impossible by rules)
        // But we check against current body minus tail if we don't grow.
        // Simplest: Check full body. If it's the tail, it's fine ONLY if we don't grow.
        
        let growing = new_pos == self.food;
        
        if self.snake.body.contains(&new_pos) {
            // If we are not growing, and new_pos is the tail, it's valid (chasing tail)
            if !growing && new_pos == *self.snake.body.back().unwrap() {
                // Safe
            } else {
                self.game_over = true;
                return GameEvent::GameOver;
            }
        }

        self.snake.body.push_front(new_pos);
        // Update direction if changed by transition
        self.snake.direction = new_dir;
        self.snake.next_direction = new_dir; // Lock it to avoid quick double turns messing up? 
        // Actually, we should probably keep next_direction as user input buffer, 
        // but if transition rotates us, we must update the current direction.
        
        if growing {
            self.score += if self.is_prize { 5 } else { 1 };
            if self.score > self.high_score {
                self.high_score = self.score;
            }
            self.food_eaten_count += 1;
            let event = if self.is_prize { GameEvent::EatPrize } else { GameEvent::Eat };
            self.spawn_food();
            event
        } else {
            self.snake.body.pop_back();
            GameEvent::None
        }
    }

    fn calculate_next_position(&self, pos: Position, dir: Direction) -> (Position, Direction) {
        let n = self.config.grid_size;
        let mut u = pos.u;
        let mut v = pos.v;
        let mut face = pos.face;
        let mut new_dir = dir;

        match dir {
            Direction::Up => v += 1,
            Direction::Down => v -= 1,
            Direction::Left => u -= 1,
            Direction::Right => u += 1,
        }

        // Check bounds and transition
        if u < 0 || u >= n || v < 0 || v >= n {
            // Transition logic
            match (face, dir) {
                // Front Transitions
                (Face::Front, Direction::Up) => { face = Face::Top; v = 0; } // Enter Bottom of Top
                (Face::Front, Direction::Down) => { face = Face::Bottom; v = n - 1; } // Enter Top of Bottom
                (Face::Front, Direction::Left) => { face = Face::Left; u = n - 1; } // Enter Right of Left
                (Face::Front, Direction::Right) => { face = Face::Right; u = 0; } // Enter Left of Right

                // Back Transitions
                (Face::Back, Direction::Up) => { face = Face::Top; v = n - 1; new_dir = Direction::Down; u = n - 1 - u; } // Top of Back connects to Top of Top? 
                // Let's define the unfolding.
                // Standard box unfolding:
                //   T
                // L F R B
                //   Bo
                
                // Front (F): u=R, v=U
                // Top (T): u=R, v=B (Back) -> No, let's keep "Up" consistent with visual up if possible?
                // If we fold T up 90 deg from F:
                // F's top edge matches T's bottom edge.
                // So F(Up) -> T(Up, entering at v=0). Correct.
                
                // Right (R):
                // F's right edge matches R's left edge.
                // F(Right) -> R(Right, entering at u=0). Correct.
                
                // Left (L):
                // F's left edge matches L's right edge.
                // F(Left) -> L(Left, entering at u=N-1). Correct.
                
                // Bottom (Bo):
                // F's bottom edge matches Bo's top edge.
                // F(Down) -> Bo(Down, entering at v=N-1). Correct.
                
                // Now the secondary connections (e.g. T -> R).
                // T is above F. R is right of F.
                // T's right edge should match R's top edge?
                // Let's trace:
                // F(N, N) is top-right corner.
                // T(N, 0) is bottom-right corner of T.
                // R(0, N) is top-left corner of R.
                // So T(Right) -> R(Down)? Or R(Left)?
                // Let's visualize the corner.
                // Moving Right on T (increasing u) goes towards the corner shared by F, T, R.
                // Crossing that edge goes onto R.
                // On R, we are at the top edge (v=N-1).
                // And we are moving "Down" (decreasing v) into R?
                // Or are we moving "Right" relative to R?
                // If R's "Up" is aligned with F's "Up", then R's top edge is shared with T's right edge.
                // So T(Right) -> R(Down)?
                // Let's assume T(Right) enters R at Top edge (v=N-1).
                // Coordinate mapping:
                // T(u=N, v) -> R(u=v?, v=N-1)?
                // Let's trace the corner vertex.
                // T(N,0) (bottom-right of T) touches F(N,N) (top-right of F) and R(0,N) (top-left of R).
                // So T(Right) at v=0 enters R at u=0, v=N-1?
                // Wait, T(Right) at v=0 is the corner.
                // T(Right) generally means u goes N.
                // If we cross u=N on T, we enter R.
                // The edge is T's right edge.
                // R's corresponding edge is R's Top edge.
                // So T(Right) -> R(Down).
                // Mapping: T(v) maps to R(u)?
                // T(N, 0) -> R(0, N-1).
                // T(N, N-1) -> R(N-1, N-1).
                // So T(Right) -> R(Down). New u = T.v. New v = N-1.
                
                (Face::Top, Direction::Right) => { face = Face::Right; u = v; v = n - 1; new_dir = Direction::Down; }
                (Face::Top, Direction::Left) => { face = Face::Left; u = n - 1 - v; v = n - 1; new_dir = Direction::Down; }
                
                (Face::Top, Direction::Up) => { face = Face::Back; v = n - 1; new_dir = Direction::Down; u = n - 1 - u; } // Top of T connects to Top of B.
                // B is back of F.
                // T(Up) -> B.
                // If we go "Up" on T (away from F), we go over the back to B.
                // B's orientation: usually "Up" on B is same global Up?
                // If so, T's top edge touches B's top edge.
                // So T(Up) -> B(Down).
                // T(u) maps to B(u)?
                // T(0, N) (top-left of T) -> B(N, N) (top-right of B)?
                // Let's assume B is "unwrapped" such that B's right is L, B's left is R.
                // Standard T-shape map:
                //   T
                // L F R
                //   Bo
                //   B (below Bo)
                // OR
                //   T
                // L F R B (strip)
                //   Bo
                // Let's stick to:
                // F neighbors: T(U), Bo(D), L(L), R(R).
                // B neighbors: T(U), Bo(D), R(L), L(R). (Wrapped around).
                // Note: B's Left is R (because R wraps around). B's Right is L.
                // B's Up is T?
                // If F(Up)->T(Bottom), then T(Top)->B(Top)? Yes.
                // So T(Up) -> B(Down).
                // Coordinate matching:
                // T(u) goes 0..N (Left to Right).
                // B(u) goes 0..N (Left to Right, looking at B).
                // If we go over the top:
                // T(0, N) is T-Left-Top corner.
                // B(0, N) is B-Left-Top corner?
                // If B is viewed from behind, "Left" is actually World Right?
                // Let's define B's u,v such that B(u,v) corresponds to the face rendering.
                // If we look at B, u increases Right, v increases Up.
                // T(Top-Left) connects to B(Top-Left)?
                // No, T(Top-Left) is World-Left-Back.
                // B(Top-Left) is World-Right-Back (if looking at B from back).
                // Wait, "Left" on B (u=0) should be adjacent to R?
                // Let's check the strip: L - F - R - B - L ...
                // F(Right) -> R(Left).
                // R(Right) -> B(Left).
                // B(Right) -> L(Left).
                // L(Right) -> F(Left).
                // So B's Left edge (u=0) is R's Right edge (u=N).
                // B's Right edge (u=N) is L's Left edge (u=0).
                // B's Top edge (v=N) is T's Top edge (v=N).
                // B's Bottom edge (v=0) is Bo's Bottom edge (v=0).
                
                // So T(Up) -> B(Down).
                // T(u) matches B(u) but inverted?
                // T(Left) is L side. T(Right) is R side.
                // B(Left) is R side. B(Right) is L side.
                // So T(Left) connects to B(Right)? No.
                // T(Left) connects to L.
                // T(Right) connects to R.
                // T(Top) connects to B(Top).
                // T(Top-Left) is corner L-T-B.
                // B(Top-Right) is corner L-T-B (since B(Right) is L).
                // So T(u=0) -> B(u=N-1).
                // T(u=N-1) -> B(u=0).
                // So u = N - 1 - u.
                
                (Face::Top, Direction::Down) => { face = Face::Front; v = n - 1; } // Back to Front

                // Bottom Transitions
                (Face::Bottom, Direction::Up) => { face = Face::Front; v = 0; }
                (Face::Bottom, Direction::Down) => { face = Face::Back; v = 0; new_dir = Direction::Up; u = n - 1 - u; } // Bo(Bottom) -> B(Bottom).
                // B(Bottom) is v=0.
                // Bo(Bottom) is v=0.
                // Bo(Down) -> B(Up).
                // Matching u:
                // Bo(Left) is L. Bo(Right) is R.
                // B(Left) is R. B(Right) is L.
                // Bo(Left-Bottom) corner L-Bo-B.
                // B(Right-Bottom) corner L-Bo-B.
                // So Bo(u=0) -> B(u=N-1).
                // u = N - 1 - u.
                
                (Face::Bottom, Direction::Left) => { face = Face::Left; u = v; v = 0; new_dir = Direction::Up; } // Bo(Left) -> L(Bottom).
                // Bo(Left) is u=-1.
                // L(Bottom) is v=0.
                // Bo(Left) edge matches L(Bottom) edge.
                // Bo(u=0, v) -> L(u=?, v=0).
                // Bo(0, 0) (Left-Bottom) -> L(N-1, 0) (Right-Bottom)?
                // Bo(0, N-1) (Left-Top) -> L(0, 0) (Left-Bottom)?
                // Wait, Bo(Top) is F. L(Bottom) is Bo.
                // L(Right) is F.
                // So L(Bottom-Right) is F-Bo-L corner.
                // Bo(Top-Left) is F-Bo-L corner.
                // So Bo(0, N-1) -> L(N-1, 0).
                // Bo(0, 0) -> L(0, 0)?
                // Let's check Bo(Bottom) -> B.
                // L(Left) -> B.
                // So L(Bottom-Left) is B-Bo-L corner.
                // Bo(Bottom-Left) is B-Bo-L corner.
                // So Bo(0, 0) -> L(0, 0).
                // So Bo(Left) -> L(Up).
                // Bo.v maps to L.u.
                // Bo(v=0) -> L(u=0).
                // Bo(v=N-1) -> L(u=N-1).
                // So u = v.
                
                (Face::Bottom, Direction::Right) => { face = Face::Right; u = n - 1 - v; v = 0; new_dir = Direction::Up; }
                // Bo(Right) -> R(Bottom).
                // Bo(Right) is u=N.
                // R(Bottom) is v=0.
                // Bo(N, N-1) (Top-Right) -> R(0, 0) (Left-Bottom).
                // Bo(N, 0) (Bottom-Right) -> R(N-1, 0) (Right-Bottom).
                // So Bo.v maps to R.u inverted.
                // u = N - 1 - v.

                // Right Transitions
                (Face::Right, Direction::Left) => { face = Face::Front; u = n - 1; }
                (Face::Right, Direction::Right) => { face = Face::Back; u = 0; } // R(Right) -> B(Left).

                // Wait, R(Up) is v=N.
                // T(Right) is u=N.
                // R(Top-Left) -> T(Bottom-Right)?
                // R(0, N) -> T(N, 0).
                // R(N-1, N) -> T(N, N-1).
                // So R.u maps to T.v.
                // u = R.u. v = R.u.
                // Wait, R(Up) -> T(Right).
                // We enter T from the Right edge (u=N).
                // So we are moving Left on T.
                // new_dir = Left.
                // u = N - 1.
                // v = ?.
                // R(0, N) -> T(N, 0). (u=0 -> v=0).
                // R(N-1, N) -> T(N, N-1). (u=N-1 -> v=N-1).
                // So v = u (old u).
                (Face::Right, Direction::Up) => { face = Face::Top; let old_u = u; u = n - 1; v = old_u; new_dir = Direction::Left; }

                (Face::Right, Direction::Down) => { face = Face::Bottom; let old_u = u; u = n - 1; v = n - 1 - old_u; new_dir = Direction::Left; }
                // R(Down) -> Bo(Right).
                // R(Bottom-Left) -> Bo(Top-Right).
                // R(0, 0) -> Bo(N, N-1).
                // R(N-1, 0) -> Bo(N, 0).
                // So R.u maps to Bo.v inverted.
                // v = N - 1 - u.
                // Enter Bo from Right (u=N). Moving Left.
                
                // Left Transitions
                (Face::Left, Direction::Right) => { face = Face::Front; u = 0; }
                (Face::Left, Direction::Left) => { face = Face::Back; u = n - 1; } // L(Left) -> B(Right).
                (Face::Left, Direction::Up) => { face = Face::Top; let old_u = u; u = 0; v = n - 1 - old_u; new_dir = Direction::Right; }
                // L(Up) -> T(Left).
                // L(Top-Right) -> T(Bottom-Left).
                // L(N-1, N) -> T(0, 0).
                // L(0, N) -> T(0, N-1).
                // So L.u maps to T.v inverted.
                // v = N - 1 - u.
                // Enter T from Left (u=-1). Moving Right.
                
                (Face::Left, Direction::Down) => { face = Face::Bottom; let old_u = u; u = 0; v = old_u; new_dir = Direction::Right; }
                // L(Down) -> Bo(Left).
                // L(Bottom-Right) -> Bo(Top-Left).
                // L(N-1, 0) -> Bo(0, N-1).
                // L(0, 0) -> Bo(0, 0).
                // So L.u maps to Bo.v.
                // v = u.
                // Enter Bo from Left (u=-1). Moving Right.
                
                // Back Transitions
                (Face::Back, Direction::Down) => { face = Face::Bottom; v = 0; new_dir = Direction::Up; u = n - 1 - u; } // B(Down) -> Bo(Bottom) ??
                // Wait, B(Down) is v=-1.
                // B(Top) is T. B(Bottom) is Bo.
                // B(Bottom) edge matches Bo(Bottom) edge.
                // So B(Down) -> Bo(Up).
                // u = N - 1 - u.
                
                (Face::Back, Direction::Left) => { face = Face::Right; u = n - 1; } // B(Left) -> R(Right).
                (Face::Back, Direction::Right) => { face = Face::Left; u = 0; } // B(Right) -> L(Left).
                

            }
        }

        (Position { face, u, v }, new_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_front_transitions() {
        let grid_size = 16;
        let game = GameState::new(grid_size);
        
        // Front -> Top
        let pos = Position { face: Face::Front, u: 5, v: 15 };
        let (new_pos, _) = game.calculate_next_position(pos, Direction::Up);
        assert_eq!(new_pos.face, Face::Top);
        assert_eq!(new_pos.u, 5);
        assert_eq!(new_pos.v, 0);

        // Front -> Right
        let pos = Position { face: Face::Front, u: 15, v: 5 };
        let (new_pos, _) = game.calculate_next_position(pos, Direction::Right);
        assert_eq!(new_pos.face, Face::Right);
        assert_eq!(new_pos.u, 0);
        assert_eq!(new_pos.v, 5);
    }

    #[test]
    fn test_top_transitions() {
        let grid_size = 16;
        let game = GameState::new(grid_size);

        // Top -> Back (Up)
        let pos = Position { face: Face::Top, u: 5, v: 15 };
        let (new_pos, new_dir) = game.calculate_next_position(pos, Direction::Up);
        assert_eq!(new_pos.face, Face::Back);
        assert_eq!(new_dir, Direction::Down);
        assert_eq!(new_pos.u, 16 - 1 - 5); // 10
        assert_eq!(new_pos.v, 15);
    }
}
