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

    // Shared state for mobile controls (Arc<Mutex<>> not needed as closure captures it, but need Interior Mutability for event listeners)
    // Since event listeners are callbacks, they can't easily share state with the main loop unless we use Rc<RefCell<>>
    // However, the main loop is a closure passed to render_loop.
    // Simple approach: Polling global variables or using specific events in the frame input if possible?
    // No, `three-d` events are from window.
    // But our buttons are HTML elements. `three-d` might not capture clicks on them if they are outside canvas?
    // Actually, we can just check a shared state that the click handlers update.

    use std::rc::Rc;
    use std::cell::RefCell;

    let mobile_input = Rc::new(RefCell::new(None));
    let mobile_input_clone = mobile_input.clone();

    // Attach listeners to buttons
    let attach_btn = |id: &str, dir: Direction| {
        let elem = document.get_element_by_id(id);
        if let Some(e) = elem {
            let input = mobile_input_clone.clone();
            let closure = Closure::wrap(Box::new(move || {
                *input.borrow_mut() = Some(dir);
            }) as Box<dyn FnMut()>);
            // Use pointerdown to be responsive
            e.add_event_listener_with_callback("pointerdown", closure.as_ref().unchecked_ref()).unwrap();
            closure.forget(); // Memory leak but fine for single page app
        }
    };

    attach_btn("btn-up", Direction::Up);
    attach_btn("btn-down", Direction::Down);
    attach_btn("btn-left", Direction::Left);
    attach_btn("btn-right", Direction::Right);

    // Swipe detection
    let swipe_start = Rc::new(RefCell::new(None));
    let swipe_start_clone = swipe_start.clone();
    let mobile_input_swipe = mobile_input.clone();

    {
        let closure = Closure::wrap(Box::new(move |e: web_sys::TouchEvent| {
            if let Some(touch) = e.touches().get(0) {
                *swipe_start_clone.borrow_mut() = Some((touch.client_x(), touch.client_y()));
            }
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback("touchstart", closure.as_ref().unchecked_ref()).unwrap();
        closure.forget();
    }

    {
        let swipe_start_move = swipe_start.clone();
        let closure = Closure::wrap(Box::new(move |e: web_sys::TouchEvent| {
             if let Some(start) = *swipe_start_move.borrow() {
                 if let Some(touch) = e.changed_touches().get(0) {
                     let dx = touch.client_x() - start.0;
                     let dy = touch.client_y() - start.1;

                     if dx.abs() > 30 || dy.abs() > 30 {
                         if dx.abs() > dy.abs() {
                             if dx > 0 { *mobile_input_swipe.borrow_mut() = Some(Direction::Right); }
                             else { *mobile_input_swipe.borrow_mut() = Some(Direction::Left); }
                         } else {
                             if dy > 0 { *mobile_input_swipe.borrow_mut() = Some(Direction::Down); }
                             else { *mobile_input_swipe.borrow_mut() = Some(Direction::Up); }
                         }
                         // Reset start to avoid continuous triggers?
                         // Or just trigger once per swipe.
                         *swipe_start_move.borrow_mut() = None;
                     }
                 }
             }
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback("touchmove", closure.as_ref().unchecked_ref()).unwrap();
        closure.forget();
    }
    // Also reset on touchend
    {
         let swipe_start_reset = swipe_start.clone();
         let closure = Closure::wrap(Box::new(move || {
            *swipe_start_reset.borrow_mut() = None;
        }) as Box<dyn FnMut()>);
        canvas.add_event_listener_with_callback("touchend", closure.as_ref().unchecked_ref()).unwrap();
        closure.forget();
    }

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
        // Check mobile input
        let mut mobile_dir = None;
        if let Ok(mut input) = mobile_input.try_borrow_mut() {
            if let Some(dir) = *input {
                mobile_dir = Some(dir);
                audio.resume_context(); // Resume on mobile interaction too
                *input = None;
            }
        }

        if let Some(dir) = mobile_dir {
             match dir {
                Direction::Up => if game.snake.direction != Direction::Down { game.snake.next_direction = Direction::Up; },
                Direction::Down => if game.snake.direction != Direction::Up { game.snake.next_direction = Direction::Down; },
                Direction::Left => if game.snake.direction != Direction::Right { game.snake.next_direction = Direction::Left; },
                Direction::Right => if game.snake.direction != Direction::Left { game.snake.next_direction = Direction::Right; },
             }
        }

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
