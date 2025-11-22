use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use three_d::*;
use crate::game::{GameState, Direction};
use crate::renderer::GameRenderer;
use crate::audio::AudioPlayer;

mod game;
mod renderer;
mod audio;

#[wasm_bindgen(start)]
pub fn init() -> Result<(), JsValue> {
    web_sys::console::log_1(&"Rust: init started".into());
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).unwrap();

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>().unwrap();

    log::info!("Found canvas, creating Window...");
    let window = Window::new(WindowSettings {
        title: "3D Snake".to_string(),
        min_size: (100, 100),
        ..Default::default()
    })
    .unwrap();
    log::info!("Window created successfully!");

    let context = window.gl();
    let grid_size = 10;
    let mut game = GameState::new(grid_size);
    let mut renderer = GameRenderer::new(context, grid_size);
    let audio = AudioPlayer::new();

    // Game loop variables
    let mut time_since_last_move = 0.0;
    let mut has_logged = false;

    // Hide loading screen
    if let Some(loading_el) = document.get_element_by_id("loading") {
        loading_el.set_attribute("style", "display: none").unwrap();
    }
    
    // Focus canvas to ensure it receives keys
    canvas.focus().unwrap_or(());
    
    window.render_loop(move |frame_input| {
        if !has_logged {
            log::info!("Viewport: {:?}", frame_input.viewport);
            has_logged = true;
        }
        let mut events = frame_input.events.clone(); // Clone events to pass to camera and handle locally
        
        // Handle Input
        for event in &events {
            if let Event::KeyPress { kind, .. } = event {
                // Resume audio context on first interaction
                audio.resume_context();

                match kind {
                    Key::ArrowUp | Key::W => {
                        if game.snake.direction != Direction::Down {
                            game.snake.next_direction = Direction::Up;
                        }
                    }
                    Key::ArrowDown | Key::S => {
                        if game.snake.direction != Direction::Up {
                            game.snake.next_direction = Direction::Down;
                        }
                    }
                    Key::ArrowLeft | Key::A => {
                        if game.snake.direction != Direction::Right {
                            game.snake.next_direction = Direction::Left;
                        }
                    }
                    Key::ArrowRight | Key::D => {
                        if game.snake.direction != Direction::Left {
                            game.snake.next_direction = Direction::Right;
                        }
                    }
                    Key::R => {
                        if game.game_over {
                            let high_score = game.high_score;
                            game = GameState::new(grid_size);
                            game.high_score = high_score;
                        }
                    }
                    _ => {}
                }
            }
        }

        // Update Camera
        renderer.update_camera(&mut events);
        renderer.resize(frame_input.viewport.width, frame_input.viewport.height);

        // Update Game Logic
        // Use accumulated time for fixed step update
        time_since_last_move += frame_input.elapsed_time / 1000.0; // elapsed_time is ms

        // Calculate current speed based on score (max speed at 50 points)
        let base_speed = 0.15;
        let min_speed = 0.05;
        let speed_reduction = (game.score as f64 * 0.002).min(base_speed - min_speed);
        let move_interval = base_speed - speed_reduction;

        if time_since_last_move >= move_interval {
            let old_food_pos = game.food;
            let event = game.update();
            match event {
                crate::game::GameEvent::Eat => {
                    audio.play_eat();
                    renderer.spawn_particles(old_food_pos, false);
                },
                crate::game::GameEvent::EatPrize => {
                    audio.play_prize();
                    renderer.spawn_particles(old_food_pos, true);
                },
                crate::game::GameEvent::GameOver => audio.play_game_over(),
                crate::game::GameEvent::None => {}
            }
            time_since_last_move = 0.0;
        }

        // Update UI
        update_ui(&game);

        // Render
        renderer.render(&game, &frame_input.screen(), frame_input.elapsed_time / 1000.0);

        FrameOutput::default()
    });

    Ok(())
}

fn update_ui(game: &GameState) {
    let document = web_sys::window().unwrap().document().unwrap();
    
    if let Some(score_el) = document.get_element_by_id("score") {
        score_el.set_inner_html(&game.score.to_string());
    }

    if let Some(high_score_el) = document.get_element_by_id("high-score") {
        high_score_el.set_inner_html(&game.high_score.to_string());
        if let Some(container) = document.get_element_by_id("high-score-container") {
             container.class_list().remove_1("hidden").unwrap_or(());
        }
    }

    if let Some(game_over_el) = document.get_element_by_id("game-over") {
        let class_list = game_over_el.class_list();
        if game.game_over {
            class_list.remove_1("hidden").unwrap();
            if let Some(final_score_el) = document.get_element_by_id("final-score") {
                final_score_el.set_inner_html(&format!("Score: {}", game.score));
            }
        } else {
            class_list.add_1("hidden").unwrap();
        }
    }
}
