use std::fmt;

fn docker_cmd(sudo: bool) -> &'static str {
    if sudo { "sudo docker" } else { "docker" }
}

pub struct DockerBuildCommand<'a> {
    sudo: bool,
    docker_file: Option<&'a str>,
    tag: &'a str,
    context_path: &'a str,
    platform: Option<&'a str>,
    output: Option<&'a str>,
}

impl<'a> DockerBuildCommand<'a> {
    pub fn new(sudo: bool, docker_file: Option<&'a str>, tag: &'a str, context_path: &'a str) -> Self {
        Self { sudo, docker_file, tag, context_path, platform: None, output: None }
    }

    pub fn with_platform(mut self, platform: &'a str) -> Self {
        self.platform = Some(platform);
        self
    }

    pub fn with_output(mut self, output: &'a str) -> Self {
        self.output = Some(output);
        self
    }
}

impl<'a> fmt::Display for DockerBuildCommand<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let platform_arg = if let Some(p) = self.platform {
            format!("--platform {} ", p)
        } else {
            String::new()
        };
        let output_arg = if let Some(o) = self.output {
            format!("--output {} ", o)
        } else {
            String::new()
        };
        let docker_file_arg = if let Some(df) = self.docker_file {
            format!("-f {} ", df)
        } else {
            String::new()
        };
        write!(f, "{} build {} {} {} -t {}:latest {}", docker_cmd(self.sudo), platform_arg, output_arg, docker_file_arg, self.tag, self.context_path)
    }
}

pub struct DockerSaveCommand<'a> {
    sudo: bool,
    image: &'a str,
    output_path: &'a str,
}

impl<'a> DockerSaveCommand<'a> {
    pub fn new(sudo: bool, image: &'a str, output_path: &'a str) -> Self {
        Self { sudo, image, output_path }
    }
}

impl<'a> fmt::Display for DockerSaveCommand<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} save -o {} {}:latest", docker_cmd(self.sudo), self.output_path, self.image)
    }
}

pub struct DockerLoadCommand<'a> {
    sudo: bool,
    input_path: &'a str,
}

impl<'a> DockerLoadCommand<'a> {
    pub fn new(sudo: bool, input_path: &'a str) -> Self {
        Self { sudo, input_path }
    }
}

impl<'a> fmt::Display for DockerLoadCommand<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} load -i {}", docker_cmd(self.sudo), self.input_path)
    }
}

pub struct DockerStopCommand<'a> {
    sudo: bool,
    container_name: &'a str,
}

impl<'a> DockerStopCommand<'a> {
    pub fn new(sudo: bool, container_name: &'a str) -> Self {
        Self { sudo, container_name }
    }
}

impl<'a> fmt::Display for DockerStopCommand<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} stop {}", docker_cmd(self.sudo), self.container_name)
    }
}

pub struct DockerRmCommand<'a> {
    sudo: bool,
    container_name: &'a str,
}

impl<'a> DockerRmCommand<'a> {
    pub fn new(sudo: bool, container_name: &'a str) -> Self {
        Self { sudo, container_name }
    }
}

impl<'a> fmt::Display for DockerRmCommand<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} rm {}", docker_cmd(self.sudo), self.container_name)
    }
}

pub struct DockerInspectCommand<'a> {
    sudo: bool,
    container_name: &'a str,
    format: &'a str,
}

impl<'a> DockerInspectCommand<'a> {
    pub fn new(sudo: bool, container_name: &'a str, format: &'a str) -> Self {
        Self { sudo, container_name, format }
    }
}

impl<'a> fmt::Display for DockerInspectCommand<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} inspect --format \"{}\" {}", docker_cmd(self.sudo), self.format, self.container_name)
    }
}

pub struct DockerStatsCommand<'a> {
    sudo: bool,
    container_name: &'a str,
    format: &'a str,
}

impl<'a> DockerStatsCommand<'a> {
    pub fn new(sudo: bool, container_name: &'a str, format: &'a str) -> Self {
        Self { sudo, container_name, format }
    }
}

impl<'a> fmt::Display for DockerStatsCommand<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} stats --no-stream --format \"{}\" {}", docker_cmd(self.sudo), self.format, self.container_name)
    }
}

pub struct DockerRunCommand<'a> {
    sudo: bool,
    image: &'a str,
    name: &'a str,
    ports: Vec<(u16, u16)>, // host, container
    env: Vec<(&'a str, String)>,
    network: Option<&'a str>,
    volumes: Vec<(&'a str, &'a str)>,
    detach: bool,
    ulimit: Option<&'a str>,
    sysctl: Vec<(&'a str, &'a str)>,
    args: Vec<&'a str>,
}

impl<'a> DockerRunCommand<'a> {
    pub fn new(sudo: bool, image: &'a str, name: &'a str) -> Self {
        Self {
            sudo,
            image,
            name,
            ports: Vec::new(),
            env: Vec::new(),
            network: None,
            volumes: Vec::new(),
            detach: true,
            ulimit: None,
            sysctl: Vec::new(),
            args: Vec::new(),
        }
    }

    pub fn port(mut self, host: u16, container: u16) -> Self {
        self.ports.push((host, container));
        self
    }

    pub fn env(mut self, key: &'a str, val: impl Into<String>) -> Self {
        self.env.push((key, val.into()));
        self
    }

    pub fn network(mut self, network: &'a str) -> Self {
        self.network = Some(network);
        self
    }

    pub fn volume(mut self, host: &'a str, container: &'a str) -> Self {
        self.volumes.push((host, container));
        self
    }

    pub fn detach(mut self, detach: bool) -> Self {
        self.detach = detach;
        self
    }

    pub fn ulimit(mut self, ulimit: &'a str) -> Self {
        self.ulimit = Some(ulimit);
        self
    }
    pub fn sysctl(mut self, key: &'a str, value: &'a str) -> Self {
        self.sysctl.push((key, value));
        self
    }

    pub fn arg(mut self, arg: &'a str) -> Self {
        self.args.push(arg);
        self
    }
}

impl<'a> fmt::Display for DockerRunCommand<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} run ", docker_cmd(self.sudo))?;
        
        if self.detach {
            write!(f, "-d ")?;
        }
        
        write!(f, "--name {} ", self.name)?;
        if let Some(ulimit) = self.ulimit {
            write!(f, "--ulimit {} ", ulimit)?;
        }

        for (k, v) in &self.sysctl {
            write!(f, "--sysctl {}={} ", k, v)?;
        }
        
        if let Some(net) = self.network {
            write!(f, "--network {} ", net)?;
        }

        for (host, container) in &self.ports {
            write!(f, "-p {}:{} ", host, container)?;
        }
        
        for (k, v) in &self.env {
            write!(f, "-e {}={} ", k, v)?;
        }
        
        for (host, container) in &self.volumes {
            write!(f, "-v {}:{} ", host, container)?;
        }
        
        write!(f, "{}:latest", self.image)?;

        for arg in &self.args {
            write!(f, " {}", arg)?;
        }

        Ok(())
    }
}
