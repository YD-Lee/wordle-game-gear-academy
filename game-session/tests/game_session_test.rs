use gstd::{prelude::*, ActorId};
use gtest::{Log, Program, System};
use game_session_io::*;

const WORDLE_ID: u64 = 1;
const GAME_SESSION_ID: u64 = 2;
const USER1: u64 = 10;
const USER2: u64 = 11;

fn setup() -> System {
    let sys = System::new();
    sys.init_logger();


    let wordle = Program::from_file(&sys, "../target/wasm32-unknown-unknown/debug/wordle.wasm");
    let game_session = Program::from_file(&sys, "../target/wasm32-unknown-unknown/debug/game_session.wasm");

    let user_id: ActorId = USER1.into();
    let wordle_id: ActorId = WORDLE_ID.into();
    assert!(!wordle.send(user_id, wordle_id).main_failed());
    assert!(!game_session.send(user_id, wordle_id).main_failed());
    return sys;
}

// #[test]
fn test_win() {
    let sys = setup();
    let game_session = sys.get_program(GAME_SESSION_ID).unwrap();

    // user1 starts game, and check words (the hidden word is "house" in test mode)
    let user1: ActorId = USER1.into();
    assert!(!game_session.send(user1, SessionAction::StartGame).main_failed());
    assert!(!game_session.send(user1, SessionAction::CheckWord { word: "human".to_string() }).main_failed());
    assert!(!game_session.send(user1, SessionAction::CheckWord { word: "house".to_string() }).main_failed());

    let state: State = game_session.read_state(b"").unwrap();
    assert_eq!(state.user_sessions.len(), 1);
    assert_eq!(state.user_sessions[0].0, user1);
    assert_eq!(state.user_sessions[0].1.check_count, 2);
    assert_eq!(state.user_sessions[0].1.status, SessionStatus::StartGameWaiting);
    assert_eq!(state.user_sessions[0].1.result, SessionResult::Win);

    // support multiple users:
    // user2 starts game, and check words (the hidden word is "house" in test mode)
    let user2: ActorId = USER2.into();
    assert!(!game_session.send(user2, SessionAction::StartGame).main_failed());
    assert!(!game_session.send(user2, SessionAction::CheckWord { word: "human".to_string() }).main_failed());
    assert!(!game_session.send(user2, SessionAction::CheckWord { word: "human".to_string() }).main_failed());
    assert!(!game_session.send(user2, SessionAction::CheckWord { word: "human".to_string() }).main_failed());
    assert!(!game_session.send(user2, SessionAction::CheckWord { word: "human".to_string() }).main_failed());
    assert!(!game_session.send(user2, SessionAction::CheckWord { word: "human".to_string() }).main_failed());
    assert!(!game_session.send(user2, SessionAction::CheckWord { word: "house".to_string() }).main_failed());

    let state: State = game_session.read_state(b"").unwrap();
    assert_eq!(state.user_sessions.len(), 2);

    let session = if user2 == state.user_sessions[0].0 {
        &state.user_sessions[0].1
    } else {
        &state.user_sessions[1].1
    };
    assert_eq!(session.check_count, 6);
    assert_eq!(session.status, SessionStatus::StartGameWaiting);
    assert_eq!(session.result, SessionResult::Lose);
}

// #[test]
fn test_lose_with_too_many_check() {
    let sys: System = setup();
    let game_session = sys.get_program(GAME_SESSION_ID).unwrap();
    
    // user starts game, and check word for >=6 times (the hidden word is "house" in test mode)
    let user: ActorId = USER1.into();
    assert!(!game_session.send(user, SessionAction::StartGame).main_failed());
    assert!(!game_session.send(user, SessionAction::CheckWord { word: "human".to_string() }).main_failed());
    assert!(!game_session.send(user, SessionAction::CheckWord { word: "human".to_string() }).main_failed());
    assert!(!game_session.send(user, SessionAction::CheckWord { word: "human".to_string() }).main_failed());
    assert!(!game_session.send(user, SessionAction::CheckWord { word: "human".to_string() }).main_failed());
    assert!(!game_session.send(user, SessionAction::CheckWord { word: "human".to_string() }).main_failed());
    assert!(!game_session.send(user, SessionAction::CheckWord { word: "human".to_string() }).main_failed());

    let state: State = game_session.read_state(b"").unwrap();
    assert_eq!(state.user_sessions.len(), 1);
    assert_eq!(state.user_sessions[0].0, user);
    assert_eq!(state.user_sessions[0].1.check_count, 6);
    assert_eq!(state.user_sessions[0].1.status, SessionStatus::StartGameWaiting);
    assert_eq!(state.user_sessions[0].1.result, SessionResult::Lose);
}

#[test]
fn test_lose_with_timeout() {
    let sys: System = setup();
    let game_session = sys.get_program(GAME_SESSION_ID).unwrap();
    
    // user starts game, and check word for >=6 times (the hidden word is "house" in test mode)
    let user: ActorId = USER1.into();
    assert!(!game_session.send(user, SessionAction::StartGame).main_failed());
    assert!(!game_session.send(user, SessionAction::CheckWord { word: "human".to_string() }).main_failed());

    sys.spend_blocks(10);

    let state: State = game_session.read_state(b"").unwrap();
    assert_eq!(state.user_sessions.len(), 1);
    assert_eq!(state.user_sessions[0].0, user);
    assert_eq!(state.user_sessions[0].1.check_count, 1);
    assert_eq!(state.user_sessions[0].1.status, SessionStatus::StartGameWaiting);
    assert_eq!(state.user_sessions[0].1.result, SessionResult::Lose);
}