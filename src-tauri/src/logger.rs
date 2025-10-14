use once_cell::sync::Lazy;
use std::sync::Mutex;
use serde::Serialize;

pub struct Logger {
    pub name: String,
    buffer: Vec<String>,
}

#[derive(Serialize)]
pub struct LogResponse {
    pub logs: Vec<String>,
    pub count: usize,
}

impl Logger {
    pub fn new(name: &str) -> Self {
        Logger {
            name: name.to_string(),
            buffer: Vec::with_capacity(1000),
        }
    }

    pub fn info(&mut self, message: &str) {
        let formatted_message = format!("[{}] {}", self.name, message);
        println!("{}", formatted_message);
        
        if self.buffer.len() >= 1000 {
            self.buffer.remove(0);
        }
        self.buffer.push(formatted_message);
    }

    pub fn get(&self) -> Vec<String> {
        self.buffer.clone()
    }
}

pub static LOGGER: Lazy<Mutex<Logger>> = Lazy::new(|| Mutex::new(Logger::new("manga4deck")));

pub fn info(message: &str) {
    LOGGER.lock().unwrap().info(message);
}
