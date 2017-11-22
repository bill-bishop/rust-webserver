use std::thread;
use std::sync::{mpsc, Arc, Mutex};

trait FnBox {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<F>) {
        (*self)()
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = Some(thread::spawn(move || {
            loop {
                let msg = {
                    if let Ok(locked) = receiver.lock() { locked.recv() } else { continue }
                };
                match msg {
                    Ok(Message::NewJob(job)) => { job.call_box() }
                    Ok(Message::Terminate) => { break }
                    _ => { continue }
                }
            }
        }));
        Worker { id, thread }
    }
}

// TODO: learn more about why I must implement some self: Box<Self> pattern to call the closure
type Job = Box<FnBox + Send>;

enum Message {
    NewJob(Job),
    Terminate
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>
}

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is less than or equal to zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);

        for worker_id in 0..size {
            workers.push(Worker::new(worker_id, receiver.clone()));
        }

        ThreadPool { workers, sender }
    }
    pub fn execute<F>(&self, f: F)
        where F: FnOnce() + Send + 'static {
        let job = Box::new(f);
        if let Err(e) = self.sender.send(Message::NewJob(job)) {
            println!("Failed to execute job: {}", e);
        };
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            if let Err(_) =  self.sender.send(Message::Terminate) {
                println!("Failed to send Termination message");
            }
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                let join_result = thread.join();
                if let Err(_) = join_result {
                    println!("Failed to join thread, #{}", worker.id);
                }
            }
        }
    }
}