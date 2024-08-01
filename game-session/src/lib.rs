#![no_std]
use gstd::{exec, msg, prelude::*, debug, ActorId, collections::HashMap};
use wordle_io::{Action, Event};
use game_session_io::*;

static mut GAME_SESSION_STATE: Option<GameSessionState> = None;
static CHECK_GAME_STATUS_DELAY: u32 = 200;

#[no_mangle]
extern "C" fn init() {
    let wordle_program = msg::load().expect("Unable to decode init");
    debug!("wordle program id: {}", wordle_program);

    unsafe {
        GAME_SESSION_STATE = Some(GameSessionState {
            wordle_program: wordle_program,
            user_to_session: HashMap::new(),
        });
    }
    msg::reply(GameSessionEvent::Initialized, 0).expect("Unable to reply init");
}

fn get_user_session<'a>(game_session: &'a GameSessionState, user: &'a ActorId) -> &'a mut Session {
    if !game_session.user_to_session.contains_key(user) {
        game_session.user_to_session.insert(*user, Session {
            start_block: 0,
            check_count: 0,
            msg_ids: (0.into(), 0.into()),
            status: SessionStatus::StartGameWaiting,
            result: SessionResult::Ongoing,
        });
    }
    let user_session = game_session.user_to_session.get_mut(user);
}

fn handle_start_game() {
    let game_session = unsafe {GAME_SESSION_STATE.as_mut().expect("GAME_SESSION_STATE is not initialized")};
    let user = msg::source();
    let user_session = get_user_session(&game_session, &user);
    debug!("handle: StartGame, status is {:x?}", user_session.status);

    match &user_session.status {
        SessionStatus::StartGameWaiting | SessionStatus::CheckWordWaiting => {
            let msg_id = msg::send(game_session.wordle_program, Action::StartGame { user }, 0)
                .expect("Error in sending `StartGame` to wordle program");
            user_session.msg_ids = (msg_id, msg::id());
            user_session.status = SessionStatus::StartGameSent;
            debug!("handle: StartGame wait");
            exec::wait();
        },
        SessionStatus::ReplyReceived(recv_event) => {
            if let GameSessionEvent::GameStarted = recv_event {
                msg::reply(GameSessionEvent::GameStarted , 0).expect("Error in sending `GameStarted` reply");
                user_session.start_block = exec::block_height();
                user_session.check_count = 0;
                user_session.msg_ids = (0.into(), 0.into());
                user_session.status = SessionStatus::CheckWordWaiting;
                user_session.result = SessionResult::Ongoing;
                msg::send_delayed(exec::program_id(), GameSessionAction::CheckGameStatus { user }, 0, CHECK_GAME_STATUS_DELAY)
                    .expect("Error in sending a delayed message");
            } else {
                panic!("Invalid ReplyReceived event: {:x?}", recv_event);
            }
        },
        _ => {
            panic!("handle: StartGame, wrong status ({:x?})", user_session.status);
        },
    }
}

fn handle_check_word(word: String) {
    let game_session = unsafe {GAME_SESSION_STATE.as_mut().expect("GAME_SESSION_STATE is not initialized")};
    let user = msg::source();
    let user_session = get_user_session(&game_session, &user);
    debug!("handle: CheckWord, status is {:x?}", user_session.status);

    match &user_session.status {
        SessionStatus::CheckWordWaiting => {
            if word.len() != 5 || !word.chars().all(|c| c.is_lowercase()) {
                panic!("Invalid word {}", word);
            }
            
            if exec::block_height() > user_session.start_block + CHECK_GAME_STATUS_DELAY {
                user_session.status = SessionStatus::StartGameWaiting;
                user_session.result = SessionResult::Lose;
                let event = GameSessionEvent::GameOver {
                    result: SessionResult::Lose
                };
                msg::reply(event, 0).expect("Error in sending CheckWord reply");
            } else {
                let msg_id = msg::send(game_session.wordle_program, Action::CheckWord { user, word }, 0)
                    .expect("Error in sending `CheckWord`  to wordle program");
                user_session.msg_ids = (msg_id, msg::id());
                user_session.status = SessionStatus::CheckWordSent;
                debug!("handle: CheckWord wait");
                exec::wait();
            }
        },
        SessionStatus::ReplyReceived(recv_event) => {
            if let GameSessionEvent::WordChecked { correct_positions, contained_in_word } = recv_event {
                user_session.check_count += 1;
                user_session.msg_ids = (0.into(), 0.into());
                if correct_positions.len() == 5 {
                    user_session.status = SessionStatus::StartGameWaiting;
                    user_session.result = SessionResult::Win;
                    let event = GameSessionEvent::GameOver {
                        result: SessionResult::Lose,
                    };
                    msg::send(user, event, 0).expect("Error in sending `GameOver(Win)` reply");
                } else if user_session.check_count >= 6 {
                    user_session.status = SessionStatus::StartGameWaiting;
                    user_session.result = SessionResult::Lose;
                    let event = GameSessionEvent::GameOver {
                        result: SessionResult::Lose,
                    };
                    msg::send(user, event, 0).expect("Error in sending `GameOver(Lose)` reply");
                } else {
                    user_session.status = SessionStatus::CheckWordWaiting;
                    user_session.result = SessionResult::Ongoing;
                    let event = GameSessionEvent::WordChecked {
                        correct_positions: correct_positions.to_vec(),
                        contained_in_word: contained_in_word.to_vec(),
                    };
                    msg::send(user, event, 0).expect("Error in sending `WordChecked` reply");
                }
            } else {
                panic!("Invalid ReplyReceived event: {:x?}", recv_event);
            }
        },
        _ => {
            panic!("handle: CheckWord, wrong status ({:x?})", user_session.status);
        },
    }
}

fn handle_check_game_status(user: &ActorId) {
    let game_session = unsafe {GAME_SESSION_STATE.as_mut().expect("GAME_SESSION_STATE is not initialized")};
    let user_session = get_user_session(&game_session, &user);
    if exec::block_height() > user_session.start_block + CHECK_GAME_STATUS_DELAY && 
        user_session.result == SessionResult::Ongoing {
        user_session.result = SessionResult::Lose;
        user_session.status = SessionStatus::StartGameWaiting;

        let event = GameSessionEvent::GameOver {
            result: SessionResult::Lose,
        };
        msg::send(*user, event, 0).expect("Error in sending a message");
    }
}

#[no_mangle]
extern "C" fn handle() {
    let action: GameSessionAction = msg::load().expect("Unable to decode handle");
    debug!("handle: payload is {:x?}", &action);

    match &action {
        GameSessionAction::StartGame => handle_start_game(),
        GameSessionAction::CheckWord { word } => handle_check_word(word.to_string()),
        GameSessionAction::CheckGameStatus { user } => handle_check_game_status(&user),
    }
}

#[no_mangle]
extern "C" fn handle_reply() {
    debug!("handle_reply");
    let game_session = unsafe {GAME_SESSION_STATE.as_mut().expect("GAME_SESSION_STATE is not initialized")};
    let reply_to = msg::reply_to().expect("Failed to query reply_to data");
    let reply_message: Event = msg::load().expect("Unable to decode wordle's reply message");
    debug!("handle_reply: reply_message {:x?}", reply_message);

    match &reply_message {
        Event::GameStarted { user } => {
            if let Some(user_session) = game_session.user_to_session.get_mut(user) {
                if reply_to == user_session.msg_ids.0 {
                    user_session.status = SessionStatus::ReplyReceived(GameSessionEvent::GameStarted);
                    exec::wake(user_session.msg_ids.1).expect("Failed to wake message");
                } else {
                    panic!("handle_reply: reply_to does not match the message id");
                }
            } else {
                panic!("handle_reply: GameStarted, non existing user");
            }
        },
        Event::WordChecked { user, correct_positions, contained_in_word } => {
            if let Some(user_session) = game_session.user_to_session.get_mut(user) {
                if reply_to == user_session.msg_ids.0 {
                    let event = GameSessionEvent::WordChecked {
                        correct_positions: correct_positions.clone(), 
                        contained_in_word: contained_in_word.clone(),
                    };
                    user_session.status = SessionStatus::ReplyReceived(event);
                    exec::wake(user_session.msg_ids.1).expect("Failed to wake message");
                } else {
                    panic!("handle_reply: reply_to does not match the message id");
                }
            } else {
                panic!("handle_reply: WordChecked, non existing user");
            }
        },
    }
}

#[no_mangle]
extern "C" fn state() {
    let game_session = unsafe { GAME_SESSION_STATE.take().expect("Unexpected error in taking state") };
    msg::reply::<State>(game_session.into(), 0)
        .expect("Failed to encode or reply with `GameSessionState` from `state()`");
}