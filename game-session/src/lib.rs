#![no_std]
use gstd::{exec, msg, prelude::*, debug, ActorId, collections::HashMap};
use wordle_io::{Action, Event};
use game_session_io::*;

static mut GAME_SESSION_STATE: Option<GameSessionState> = None;

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

    // GAME_SESSION_STATE = Some(GameSessionState {
    //     wordle_program: wordle_program,
    //     user_to_session: HashMap::new(),
    // });
    msg::reply(GameSessionEvent::Initialized, 0).expect("Unable to reply init");
}

// fn get_user_session(game_session: &GameSessionState, user: &ActorId) -> &mut Session {
//     if !game_session.user_to_session.contains_key(user) {
//         game_session.user_to_session.insert(*user, Session {
//             start_block: 0,
//             check_count: 0,
//             msg_ids: (0, 0),
//             status: SessionStatus::StartGameWaiting,
//             result: SessionResult::Ongoing,
//         });
//     }
//     let user_session = game_session.user_to_session.get_mut(user);
// }

#[no_mangle]
extern "C" fn handle() {
    let action: GameSessionAction = msg::load().expect("Unable to decode handle");
    debug!("handle: payload is {:x?}", &action);

    let game_session = unsafe {GAME_SESSION_STATE.as_mut().expect("GAME_SESSION_STATE is not initialized")};

    let user = msg::source();
    game_session.user_to_session.insert(user, Session {
        start_block: exec::block_height(),
        check_count: 3,
        msg_ids: (11.into(), 22.into()),
        status: SessionStatus::StartGameWaiting,
        result: SessionResult::Ongoing,
    });

    // match action {
    //     GameSessionAction::StartGame => {
    //         let user = msg::source();
    //         let user_session = get_user_session(&game_session, &user);
    //         debug!("handle: StartGame, status is {}", user_session.status);

    //         match &user_session.status {
    //             SessionStatus::StartGameWaiting | SessionStatus::CheckWordWaiting => {
    //                 let msg_id = msg::send(game_session.wordle_program, Action::StartGame { user }, 0)
    //                     .expect("Error in sending `StartGame` to wordle program");
    //                 user_session.msg_ids = (msg_id, msg::id());
    //                 user_session.status = SessionStatus::StartGameSent;
    //                 debug!("handle: StartGame wait");
    //                 exec::wait();
    //             },
    //             SessionStatus::ReplyReceived(GameStarted) => {
    //                 msg::reply(GameSessionEvent::GameStarted, 0).expect("Error in sending StartGame reply");
    //                 user_session.start_block = exec::block_height();
    //                 user_session.check_count = 0;
    //                 user_session.msg_ids = ();
    //                 user_session.status = SessionStatus::CheckWordWaiting;
    //                 user_session.result = SessionResult::Ongoing;
    //                 msg::send_delayed(exec::program_id(), GameSessionAction::CheckGameStatus(user), 0, 200)
    //                     .expect("Error in sending a delayed message");
    //             },
    //             _ => {
    //                 panic!("handle: StartGame, wrong status ({})", user_session.status);
    //             },
    //         }
    //     },
    //     GameSessionAction::CheckWord { word } => {
    //         let user = msg::source();
    //         let user_session = get_user_session(&game_session, &user);
    //         debug!("handle: CheckWord, status is {}", user_session.status);

    //         match &user_session.status {
    //             SessionStatus::CheckWordWaiting => {
    //                 if word.len() != 5 || !word.chars().all(|c| c.is_lowercase()) {
    //                     panic!("Invalid word {}", word);
    //                 }
                    
    //                 if exec::block_height() > user_session.start_block + 200 {
    //                     user_session.status = SessionStatus::Lose;
    //                     msg::reply(GameSessionEvent::GameOver(SessionResult::Lose), 0).expect("Error in sending CheckWord reply");
    //                 } else {
    //                     let msg_id = msg::send(game_session.wordle_program, Action::CheckWord { user, word }, 0)
    //                         .expect("Error in sending `CheckWord`  to wordle program");
    //                     user_session.msg_ids = (msg_id, msg::id());
    //                     user_session.status = SessionStatus::CheckWordSent;
    //                     debug!("handle: CheckWord wait");
    //                     exec::wait();
    //                 }
    //             },
    //             SessionStatus::ReplyReceived(WordChecked) => {
    //                 // 检查是否猜对

                    
    //                 user_session.check_count += 1;
    //                 user_session.msg_ids = ();
    //                 user_session.status = SessionStatus::CheckWordWaiting;

    //                 if user_session.check_count >= 6 {
    //                     msg::reply(GameSessionEvent::GameOver(SessionResult::Lose), 0).expect("Error in sending CheckWord reply");
    //                     return;
    //                 }
    //             },
    //             _ => {
    //                 panic!("handle: CheckWord, wrong status ({})", user_session.status);
    //             },
    //         }
    //     },
    //     GameSessionAction::CheckGameStatus { user } => {
    //         let user_session = get_user_session(&game_session, &user);
    //         if exec::block_height() > user_session.start_block + 200 && user_session.result == SessionResult::Ongoing {
    //             user_session.result = SessionResult::Lose;
    //             user_session.status = SessionStatus::StartGameWaiting;
    //             msg::send(user, GameSessionEvent::GameOver{ user, SessionResult::Lose }, 0)
    //                 .expect("Error in sending a message");
    //         }
    //     },
    // }
}

#[no_mangle]
extern "C" fn handle_reply() {
    debug!("handle_reply");
    let game_session = unsafe {GAME_SESSION_STATE.as_mut().expect("GAME_SESSION_STATE is not initialized")};
    let reply_message: Event = msg::load().expect("Unable to decode wordle's reply message");
    debug!("handle_reply: reply_message {:?}", reply_message);

    // match &reply_message {
    //     Action::GameStarted { user } => {
    //         if !game_session.user_to_session.contains(user) {
    //             panic!("Non existing user");
    //         }
    //         let user_session = game_session.user_to_session.get_mut(user);
    //         user_session.status = SessionStatus::ReplyReceived(GameSessionEvent::GameStarted{user});
    //     },
    //     Action::WordChecked { user, correct_positions, contained_in_word } => {
    //         if !game_session.user_to_session.contains(user) {
    //             panic!("Non existing user");
    //         }
    //         let user_session = game_session.user_to_session.get_mut(user);
    //         user_session.status = SessionStatus::ReplyReceived(GameSessionEvent::WordChecked{user, correct_positions, contained_in_word});
    //     },
    // }
}

#[no_mangle]
extern "C" fn state() {
    
    let game_session = unsafe { GAME_SESSION_STATE.take().expect("Unexpected error in taking state") };
    msg::reply::<State>(game_session.into(), 0)
        .expect("Failed to encode or reply with `GameSessionState` from `state()`");
}