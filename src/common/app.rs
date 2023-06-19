#[derive(Debug, Clone)]
pub struct App {
    pub name: String,
    pub command: String,
    pub icon: Option<String>,

    pub app_id: Option<String>,
    pub is_flatpak: bool,

    pub pids: Vec<libc::pid_t>,
}

impl App {
    #[allow(dead_code)]
    pub fn serialize<W: std::io::Write>(&self, output: &mut W) -> std::io::Result<()> {
        output.write(to_binary(&self.name.len()))?;
        output.write(self.name.as_bytes())?;
        // output.write(to_binary(&self.command.len()))?;
        // output.write(self.command.as_bytes())?;
        // if self.icon.is_some() {
        //     output.write(to_binary(&true))?;
        //     let icon = self.icon.as_ref().unwrap();
        //     output.write(to_binary(&icon.len()))?;
        //     output.write(icon.as_bytes())?;
        // } else {
        //     output.write(to_binary(&false))?;
        // }
        // if self.app_id.is_some() {
        //     output.write(to_binary(&true))?;
        //     let app_id = self.app_id.as_ref().unwrap();
        //     output.write(to_binary(&app_id.len()))?;
        //     output.write(app_id.as_bytes())?;
        // } else {
        //     output.write(to_binary(&false))?;
        // }
        // output.write(to_binary(&self.is_flatpak))?;
        // output.write(to_binary(&self.pids.len()))?;
        // for pid in &self.pids {
        //     output.write(to_binary(&pid))?;
        // }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn deserialize<R: std::io::Read>(input: &mut R) -> std::io::Result<App> {
        let mut this = Self {
            name: "".to_string(),
            command: "".to_string(),
            icon: None,
            app_id: None,
            is_flatpak: false,
            pids: vec![],
        };

        let mut len = 0_usize;

        input.read_exact(to_binary_mut(&mut len))?;
        let mut name = vec![0; len];
        input.read_exact(&mut name)?;
        this.name = unsafe { String::from_utf8_unchecked(name) };

        // input.read_exact(to_binary_mut(&mut len))?;
        // let mut command = vec![0; len];
        // input.read_exact(&mut command)?;
        // this.command = unsafe { String::from_utf8_unchecked(command) };
        //
        // let mut has_icon = false;
        // input.read_exact(to_binary_mut(&mut has_icon))?;
        // if has_icon {
        //     input.read_exact(to_binary_mut(&mut len))?;
        //     let mut icon = vec![0; len];
        //     input.read_exact(&mut icon)?;
        //     this.icon = Some(unsafe { String::from_utf8_unchecked(icon) });
        // } else {
        //     this.icon = None;
        // }
        //
        // let mut has_app_id = false;
        // input.read_exact(to_binary_mut(&mut has_app_id))?;
        // if has_app_id {
        //     input.read_exact(to_binary_mut(&mut len))?;
        //     let mut app_id = vec![0; len];
        //     input.read_exact(&mut app_id)?;
        //     this.app_id = Some(unsafe { String::from_utf8_unchecked(app_id) });
        // } else {
        //     this.app_id = None;
        // }
        //
        // input.read_exact(to_binary_mut(&mut this.is_flatpak))?;
        //
        // input.read_exact(to_binary_mut(&mut len))?;
        // this.pids = Vec::with_capacity(len);
        // for _ in 0..len {
        //     let mut pid = 0;
        //     input.read_exact(to_binary_mut(&mut pid))?;
        //     this.pids.push(pid);
        // }

        Ok(this)
    }
}
