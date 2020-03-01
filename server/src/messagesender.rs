use std::io::Write;
use std::sync::mpsc::{channel, Sender};
use std::thread;

use serde_json::Value;

use super::protocol::{self, NotificationMessage, ResponseError};

#[derive(Debug)]
struct SuccessResponse {
    id: u64,
    result: Value,
}

#[derive(Debug)]
struct ErrorResponse {
    id: u64,
    err: ResponseError,
}

#[derive(Debug)]
enum SendingMessage {
    SuccessResponse(SuccessResponse),
    ErrorResponse(ErrorResponse),
    Notification(NotificationMessage),
}

#[derive(Clone)]
pub(crate) struct MessageSender {
    sender: Sender<SendingMessage>,
}

impl MessageSender {
    pub(crate) fn send_success_response(&self, id: u64, res: Value) {
        log::debug!("[send] Success: id = {}", id);
        let msg = SendingMessage::SuccessResponse(SuccessResponse {
            id: id,
            result: res,
        });
        self.send(msg);
    }

    pub(crate) fn send_error_response(&self, id: u64, err: ResponseError) {
        log::debug!("[send] Error: message = '{}'", err.message);
        let msg = SendingMessage::ErrorResponse(ErrorResponse { id: id, err: err });
        self.send(msg);
    }

    pub(crate) fn send_notification(&self, notif: NotificationMessage) {
        log::debug!("[send] {}", notif.method);
        let msg = SendingMessage::Notification(notif);
        self.send(msg);
    }

    fn send(&self, msg: SendingMessage) {
        // TODO: Make sure using unwrap() makes sense.
        self.sender.send(msg).unwrap();
    }
}

pub(crate) struct MessageSenderThread {
    sender: Sender<SendingMessage>,
    handle: thread::JoinHandle<()>,
}

impl MessageSenderThread {
    #[allow(unused)]
    pub(crate) fn join(self) {
        self.handle.join().unwrap();
    }

    pub(crate) fn get_sender(&self) -> MessageSender {
        MessageSender {
            sender: self.sender.clone(),
        }
    }
}

pub(crate) fn start_message_sender_thread<W: Write + Send + 'static>(
    mut writer: W,
) -> MessageSenderThread {
    let (sender, receiver) = channel();
    let handle = thread::spawn(move || loop {
        let msg = receiver.recv();
        // Terminate the thread when recv() failed.
        let msg = if let Ok(msg) = msg { msg } else { break };

        match msg {
            SendingMessage::SuccessResponse(res) => {
                protocol::write_success_response(&mut writer, res.id, res.result).unwrap();
            }
            SendingMessage::ErrorResponse(res) => {
                protocol::write_error_response(&mut writer, res.id, res.err).unwrap();
            }
            SendingMessage::Notification(notif) => {
                protocol::write_notification(&mut writer, &notif.method, notif.params).unwrap();
            }
        };
    });

    MessageSenderThread {
        sender: sender,
        handle: handle,
    }
}
