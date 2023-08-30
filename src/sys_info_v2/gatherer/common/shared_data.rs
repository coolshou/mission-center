use super::*;

#[allow(dead_code)]
#[derive(Debug)]
pub enum SharedDataContent {
    Monostate,
    Processes(Processes),
    Apps(Apps),
    AppPIDs(AppPIDs),
    CpuStaticInfo(CpuStaticInfo),
}

#[derive(Debug)]
pub struct SharedData {
    pub content: SharedDataContent,
}

#[allow(dead_code)]
impl SharedData {
    pub fn clear(&mut self) {
        self.content = SharedDataContent::Monostate;
    }
}
