use std::{
    sync::mpsc::{channel, Receiver, Sender, TryIter},
    thread::JoinHandle,
};

pub trait ExitType {
    fn exit() -> Self;
}

pub struct ThreadHandle<SendMsg: Send + 'static + ExitType, RecvMsg: Send + 'static> {
    channel: ThreadChannel<SendMsg, RecvMsg>,
    handle: Option<JoinHandle<()>>,
}

impl<SendMsg: Send + 'static + ExitType, RecvMsg: Send + 'static> ThreadHandle<SendMsg, RecvMsg> {
    pub fn send(&self, msg: SendMsg) {
        self.channel.send(msg);
    }
    pub fn recv(&self) -> TryIter<RecvMsg> {
        self.channel.recv()
    }
    pub fn join(&mut self) {
        self.handle.take().map(|h| h.join());
    }
    pub fn spawn<F: FnOnce(ThreadChannel<RecvMsg, SendMsg>) + Send + 'static>(f: F) -> Self {
        let (hs, tr) = channel();
        let (ts, hr) = channel();

        let th = ThreadChannel { send: ts, recv: tr };
        let run = || {
            f(th);
        };
        let handle = std::thread::spawn(run);
        ThreadHandle {
            channel: ThreadChannel { send: hs, recv: hr },
            handle: Some(handle),
        }
    }
}

impl<SendMsg: ExitType + Send, RecvMsg: Send + 'static> Drop for ThreadHandle<SendMsg, RecvMsg> {
    fn drop(&mut self) {
        let _ = self.channel.send.send(SendMsg::exit());
        self.join();
    }
}

pub struct ThreadChannel<SendMsg, RecvMsg> {
    send: Sender<SendMsg>,
    recv: Receiver<RecvMsg>,
}

impl<SendMsg, RecvMsg> ThreadChannel<SendMsg, RecvMsg> {
    pub fn sender(&self) -> Sender<SendMsg> {
        self.send.clone()
    }
    pub fn send(&self, msg: SendMsg) {
        // TODO: handle this properly
        self.send.send(msg).expect("Failed to send message");
    }
    pub fn recv(&self) -> TryIter<RecvMsg> {
        self.recv.try_iter()
    }
    pub fn recv_wait(&self) -> RecvMsg {
        self.recv.recv().expect("OOOAAAAAAA")
    }
}
