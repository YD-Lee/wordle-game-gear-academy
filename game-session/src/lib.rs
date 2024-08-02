#![no_std]
use gstd::{exec, msg, prelude::*, debug, ActorId, collections::HashMap};
use wordle_io::{Action, Event};
use game_session_io::*;

static mut GAME_SESSION_STATE: Option<GameSessionState> = None;
const MAX_CHECK_COUNT: u8 = 6;
#[cfg(test)]
const CHECK_GAME_STATUS_DELAY: u32 = 10;
#[cfg(not(test))]
const CHECK_GAME_STATUS_DELAY: u32 = 200;

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
    msg::reply(SessionEvent::Initialized, 0).expect("Unable to reply init");
}

fn handle_start_game() {
    let state = unsafe {GAME_SESSION_STATE.as_mut().expect("GAME_SESSION_STATE is not initialized")};
    let user = msg::source();
    if !state.user_to_session.contains_key(&user) {
        state.user_to_session.insert(user, Session {
            start_block: 0,
            check_count: 0,
            msg_ids: (0.into(), 0.into()),
            status: SessionStatus::StartGameWaiting,
            result: SessionResult::Ongoing,
        });
    }

    let session: &mut Session = state.user_to_session.get_mut(&user).unwrap();
    debug!("handle_start_game: status is {:x?}", session.status);
    match &session.status {
        SessionStatus::StartGameWaiting | SessionStatus::CheckWordWaiting => {
            let msg_id: gstd::MessageId = msg::send(state.wordle_program, Action::StartGame { user }, 0)
                .expect("handle_start_game: error in sending `Action::StartGame`");
            session.msg_ids = (msg_id, msg::id());
            session.status = SessionStatus::StartGameSent;

            debug!("handle_start_game: `StartGame` wait");
            exec::wait();
        },
        SessionStatus::ReplyReceived(recv_event) => {
            if let SessionEvent::GameStarted = recv_event {
                session.start_block = exec::block_height();
                session.check_count = 0;
                session.msg_ids = (0.into(), 0.into());
                session.status = SessionStatus::CheckWordWaiting;
                session.result = SessionResult::Ongoing;
                msg::reply(SessionEvent::GameStarted , 0).expect("Error in sending `GameStarted` reply");
                debug!("send delayed message, program={}, user={}", exec::program_id(), user);
                msg::send_delayed(exec::program_id(), SessionAction::CheckGameStatus { user }, 0, CHECK_GAME_STATUS_DELAY)
                    .expect("handle_start_game: error in sending `SessionAction::CheckGameStatus`");
            } else {
                panic!("handle_start_game: invalid received event: {:x?}", recv_event);
            }
        },
        _ => panic!("handle_start_game: wrong status, {:x?}", session.status),
    }
}

fn handle_check_word(word: String) {
    let state = unsafe {GAME_SESSION_STATE.as_mut().expect("GAME_SESSION_STATE is not initialized")};
    let user = msg::source();
    if !state.user_to_session.contains_key(&user) {
        panic!("handle_check_word: non-existing user");    
    }

    let session: &mut Session = state.user_to_session.get_mut(&user).unwrap();
    debug!("handle_check_word: status is {:x?}", session.status);

    match &session.status {
        SessionStatus::CheckWordWaiting => {
            if word.len() != 5 || !word.chars().all(|c| c.is_lowercase()) {
                panic!("handle_check_word: invalid word, {}", word);
            }
            
            session.check_count += 1;
            if session.check_count > MAX_CHECK_COUNT || exec::block_height() > session.start_block + CHECK_GAME_STATUS_DELAY {
                session.status = SessionStatus::StartGameWaiting;
                session.result = SessionResult::Lose;
                msg::reply(SessionEvent::GameOver { result: SessionResult::Lose }, 0)
                    .expect("handle_check_word: error in replying `SessionEvent::GameOver`");
            } else {
                let msg_id = msg::send(state.wordle_program, Action::CheckWord { user, word }, 0)
                    .expect("handle_check_word: error in sending `Action::CheckWord`");
                session.msg_ids = (msg_id, msg::id());
                session.status = SessionStatus::CheckWordSent;

                debug!("handle_check_word: `CheckWord` wait");
                exec::wait();
            }
        },
        SessionStatus::ReplyReceived(recv_event) => {
            if let SessionEvent::WordChecked { correct_positions, contained_in_word } = recv_event {

                session.msg_ids = (0.into(), 0.into());
                if correct_positions.len() == 5 {
                    session.status = SessionStatus::StartGameWaiting;
                    session.result = SessionResult::Win;
                    msg::reply(SessionEvent::GameOver { result: SessionResult::Lose, }, 0)
                        .expect("handle_check_word: error in replying `GameOver(Win)`");
                } else if session.check_count >= 6 {
                    session.status = SessionStatus::StartGameWaiting;
                    session.result = SessionResult::Lose;
                    msg::reply(SessionEvent::GameOver { result: SessionResult::Lose }, 0)
                        .expect("handle_check_word: error in replying `GameOver(Lose)`");
                } else {
                    let event = SessionEvent::WordChecked {
                        correct_positions: correct_positions.to_vec(),
                        contained_in_word: contained_in_word.to_vec(),
                    };
                    session.status = SessionStatus::CheckWordWaiting;
                    session.result = SessionResult::Ongoing;
                    msg::reply(event, 0).expect("handle_check_word: error in replying `WordChecked`");
                }
            } else {
                panic!("handle_check_word: invalid ReplyReceived event: {:x?}", recv_event);
            }
        },
        _ => panic!("handle_check_word: wrong status ({:x?})", session.status),
    }
}

fn handle_check_game_status(user: &ActorId) {
    debug!("handle_check_game_status");
    let state = unsafe {GAME_SESSION_STATE.as_mut()
        .expect("handle_check_game_status: GAME_SESSION_STATE is not initialized")};
    if !state.user_to_session.contains_key(user) {
        panic!("handle_check_game_status: non-existing user");    
    }

    let session: &mut Session = state.user_to_session.get_mut(user).unwrap();
    debug!("handle_check_game_status: block_height={}, start_block={}", exec::block_height(), session.start_block);
    if exec::block_height() > session.start_block + CHECK_GAME_STATUS_DELAY && session.result == SessionResult::Ongoing {
        session.result = SessionResult::Lose;
        session.status = SessionStatus::StartGameWaiting;
        msg::send(*user, SessionEvent::GameOver { result: SessionResult::Lose, }, 0)
            .expect("handle_check_game_status: error in sending `GameOver(Lose)`");
    }
}

#[no_mangle]
extern "C" fn handle() {
    let action: SessionAction = msg::load().expect("handle: unable to decode handle");
    debug!("handle: action is {:x?}", &action);

    match &action {
        SessionAction::StartGame => handle_start_game(),
        SessionAction::CheckWord { word } => handle_check_word(word.to_string()),
        SessionAction::CheckGameStatus { user } => handle_check_game_status(&user),
    }
}

#[no_mangle]
extern "C" fn handle_reply() {
    debug!("handle_reply");
    let state = unsafe {GAME_SESSION_STATE.as_mut().expect("GAME_SESSION_STATE is not initialized")};
    let reply_to = msg::reply_to().expect("Failed to query reply_to data");
    let reply_message: Event = msg::load().expect("Unable to decode wordle's reply message");
    debug!("handle_reply: reply_message {:x?}", reply_message);

    match &reply_message {
        Event::GameStarted { user } => {
            if let Some(session) = state.user_to_session.get_mut(user) {
                if reply_to == session.msg_ids.0 {
                    session.status = SessionStatus::ReplyReceived(SessionEvent::GameStarted);
                    exec::wake(session.msg_ids.1).expect("Failed to wake message");
                } else {
                    panic!("handle_reply: reply_to does not match the message id");
                }
            } else {
                panic!("handle_reply: GameStarted, non existing user");
            }
        },
        Event::WordChecked { user, correct_positions, contained_in_word } => {
            if let Some(session) = state.user_to_session.get_mut(user) {
                if reply_to == session.msg_ids.0 {
                    let event = SessionEvent::WordChecked {
                        correct_positions: correct_positions.clone(), 
                        contained_in_word: contained_in_word.clone(),
                    };
                    session.status = SessionStatus::ReplyReceived(event);
                    exec::wake(session.msg_ids.1).expect("Failed to wake message");
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