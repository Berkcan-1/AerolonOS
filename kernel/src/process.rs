extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProcessState {
    Running,
    Sleeping,
    Stopped,
}

pub struct ProcessControlBlock {
    pub pid: usize,
    pub name: String,
    pub state: ProcessState,
    pub ticks: u64,
}

pub struct ProcessManager {
    pub processes: Vec<ProcessControlBlock>,
    next_pid: usize,
}

impl ProcessManager {
    pub fn new() -> Self {
        let mut pm = ProcessManager {
            processes: Vec::new(),
            next_pid: 1,
        };

        pm.register_process("SystemInit");
        pm.register_process("NetPollerService");
        pm.register_process("StorageMonitor");

        pm
    }

    pub fn register_process(&mut self, name: &str) -> usize {
        let pid = self.next_pid;
        self.next_pid += 1;
        self.processes.push(ProcessControlBlock {
            pid,
            name: String::from(name),
            state: ProcessState::Running,
            ticks: 0,
        });
        pid
    }

    pub fn poll_all(&mut self) {
        for proc in &mut self.processes {
            proc.ticks += 1;
        }
        nc_net::global().poll();
    }
}
