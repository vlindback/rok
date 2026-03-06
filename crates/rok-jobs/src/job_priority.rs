// job_priority.rs

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum JobPriority {
    High,
    Normal,
    Low,
}

impl JobPriority {
    pub const COUNT: usize = 3;
    pub const ALL: [JobPriority; 3] = [JobPriority::High, JobPriority::Normal, JobPriority::Low];

    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }
}
