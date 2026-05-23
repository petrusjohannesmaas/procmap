use clap::Parser;

#[derive(Debug)]
struct Permissions {
    read: bool,
    write: bool,
    execute: bool,
    shared: bool,
}

#[derive(Debug)]
struct MemoryRegion {
    start_address: u64,
    end_address: u64,
    permissions: Permissions,
    offset: u64,
    device: String,
    inode: u64,
    path: Option<String>,
}

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    pid: Option<u32>,
    #[arg(short, long)]
    name: Option<String>,
}

fn parse_permissions(s: &str) -> Permissions {
    let chars: Vec<char> = s.chars().collect();
    Permissions {
        read: chars[0] == 'r',
        write: chars[1] == 'w',
        execute: chars[2] == 'x',
        shared: chars[3] == 's',
    }
}

fn parse_maps(contents: &str) -> Vec<MemoryRegion> {
    let mut regions = Vec::new();
    for line in contents.lines() {
        let columns: Vec<&str> = line.split_whitespace().collect();
        if columns.len() < 5 {
            continue;
        }
        let addrs: Vec<&str> = columns[0].split('-').collect();
        let start_address = u64::from_str_radix(addrs[0], 16).unwrap_or(0);
        let end_address = u64::from_str_radix(addrs[1], 16).unwrap_or(0);
        let permissions = parse_permissions(columns[1]);
        let offset = u64::from_str_radix(columns[2], 16).unwrap_or(0);
        let device = columns[3].to_string();
        let inode = columns[4].parse::<u64>().unwrap_or(0);
        let path = columns.get(5).map(|s| s.to_string());

        regions.push(MemoryRegion {
            start_address,
            end_address,
            permissions,
            offset,
            device,
            inode,
            path,
        });
    }
    regions
}

fn resolve_pid(args: &Args) -> u32 {
    if let Some(pid) = args.pid {
        return pid;
    }
    if let Some(ref name) = args.name {
        let output = std::fs::read_dir("/proc")
            .unwrap()
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().to_str()?.parse::<u32>().ok().map(|p| (p, e)))
            .find_map(|(pid, _)| {
                let comm = std::fs::read_to_string(format!("/proc/{}/comm", pid)).ok()?;
                if comm.trim() == name.as_str() {
                    Some(pid)
                } else {
                    None
                }
            });
        if let Some(pid) = output {
            return pid;
        }
        eprintln!("error: process '{}' not found", name);
        std::process::exit(1);
    }
    eprintln!("error: provide --pid or --name");
    std::process::exit(1);
}

fn print_regions(regions: &[MemoryRegion]) {
    println!(
        "{:<20} {:<6} {:<12} {:<10} {:<10} {}",
        "address range", "perms", "offset", "device", "inode", "path"
    );
    println!("{}", "-".repeat(80));
    for r in regions {
        let perms = format!(
            "{}{}{}{}",
            if r.permissions.read { 'r' } else { '-' },
            if r.permissions.write { 'w' } else { '-' },
            if r.permissions.execute { 'x' } else { '-' },
            if r.permissions.shared { 's' } else { 'p' },
        );
        let range = format!("{:x}-{:x}", r.start_address, r.end_address);
        let path = r.path.as_deref().unwrap_or("[anonymous]");
        println!(
            "{:<20} {:<6} {:<12} {:<10} {:<10} {}",
            range, perms, r.offset, r.device, r.inode, path
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let pid = resolve_pid(&args);
    let contents = std::fs::read_to_string(format!("/proc/{}/maps", pid))?;
    let regions = parse_maps(&contents);
    print_regions(&regions);
    Ok(())
}
