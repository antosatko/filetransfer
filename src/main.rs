use std::{
    fs::OpenOptions,
    io::{Read, Write},
    net::TcpStream,
    path::Path,
    time::Instant,
};

use utils::{parse_args, ProgressBar};

const LENGTH: &str = "Content-Length: ";

fn main() {
    let args = parse_args();

    let connection = TcpStream::connect(&args.address).unwrap();
    let mut client = Client { connection };

    let start_time = Instant::now();
    client.write(&Requests::Full);

    let response = client.wait_response().unwrap();
    let target_len = response.headers.content_length().unwrap();
    let mut progress = ProgressBar::new(target_len, response.data().len());

    let mut data = WholeData {
        current_len: response.data().len(),
        target_len,
        data: vec![response],
    };

    while data.current_len != data.target_len {
        client.reconnect().unwrap();
        client
            .write(&Requests::Range(data.current_len, data.target_len))
            .unwrap();
        let response = client.wait_response().unwrap();
        progress.update(response.data().len());
        if data.add(response) {
            break;
        }
    }
    progress.done();

    println!("Download complete, time: {:?}", start_time.elapsed());
    data.save(&args.output).unwrap();
    println!("Data written to {:?}", args.output);

    println!(
        "Please manually compare the SHA-256 hash printed by the server with the downloaded file"
    );
}

struct Client {
    pub connection: TcpStream,
}

struct WholeData {
    pub target_len: usize,
    pub current_len: usize,
    pub data: Vec<Response>,
}
enum Requests {
    Full,
    Range(usize, usize),
}

struct Headers {
    pub all: String,
}

struct Response {
    pub full_data: Vec<u8>,
    pub headers_splitoff: usize,
    pub headers: Headers,
}

impl Client {
    pub fn write(&mut self, request: &Requests) -> Option<()> {
        let header = request.to_header();
        let data = header.as_bytes();

        self.connection.write_all(data).ok()?;
        self.connection.flush().ok()?;
        Some(())
    }

    pub fn wait_response(&mut self) -> Option<Response> {
        let mut response = Vec::new();
        self.connection.read_to_end(&mut response).unwrap();

        response.try_into().ok()
    }

    pub fn reconnect(&mut self) -> Option<()> {
        self.connection = TcpStream::connect("127.0.0.1:8080").ok()?;
        Some(())
    }
}

impl WholeData {
    /// Returns true if the whole data has been obtained
    pub fn add(&mut self, response: Response) -> bool {
        self.current_len += response.data().len();
        self.data.push(response);
        self.current_len == self.target_len
    }

    pub fn _to_vec(&self) -> Vec<u8> {
        self.data
            .iter()
            .map(|d| d.data())
            .fold(Vec::with_capacity(self.target_len), |mut a, b| {
                a.extend_from_slice(b);
                a
            })
    }

    pub fn save<T: AsRef<Path>>(&self, path: T) -> Option<()> {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .ok()?;
        for data in &self.data {
            assert_eq!(file.write(data.data()).ok()?, data.data().len());
        }
        Some(())
    }
}


impl Requests {
    pub fn to_header(&self) -> String {
        match self {
            Self::Full => "GET / HTTP/1.0\r\n\r\n".to_string(),
            Self::Range(n, m) => format!("GET / HTTP/1.0\r\nRange: bytes={n}-{m}\r\n\r\n"),
        }
    }
}

impl Headers {
    pub fn content_length(&self) -> Option<usize> {
        self.all.lines().find_map(|l| {
            l.starts_with(LENGTH)
                .then(|| l.split_at(LENGTH.len()).1.parse().ok())?
        })
    }
}

impl TryFrom<Vec<u8>> for Response {
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if let Some(pos) = value.windows(4).position(|w| w == b"\r\n\r\n") {
            let headers = match String::from_utf8(value[..pos].to_vec()) {
                Ok(v) => Headers { all: v },
                Err(_) => return Err(()),
            };
            Ok(Self {
                full_data: value,
                headers_splitoff: pos + 4,
                headers,
            })
        } else {
            Err(())
        }
    }

    type Error = ();
}

impl Response {
    pub fn data(&self) -> &[u8] {
        &self.full_data[self.headers_splitoff..]
    }
}

mod utils {
    use std::{env, io::Write, path::PathBuf};

    pub struct ProgressBar {
        pub target: usize,
        pub current: usize,
    }

    impl ProgressBar {
        const WIDTH: usize = 32;

        pub fn new(target: usize, current: usize) -> Self {
            println!("Downloading {:.2}Kb", Self::to_kb(target));
            let mut this = ProgressBar { target, current: 0 };
            this.update(current);
            this
        }

        pub fn update(&mut self, add: usize) {
            self.print(add);
            self.current += add;
        }

        pub fn done(&mut self) {
            self.current = self.target;
            println!(
                "\r|{}| {:.2}Kb             ",
                "▓".repeat(Self::WIDTH - 1),
                Self::to_kb(self.target)
            )
        }

        fn print(&self, add: usize) {
            let done_size =
                (self.current as f32 / self.target as f32 * Self::WIDTH as f32).floor() as usize;
            let added_size =
                (add as f32 / self.target as f32 * Self::WIDTH as f32).floor() as usize;
            let remaining_size = Self::WIDTH.saturating_sub(done_size + added_size + 1);
            let done = "▓".repeat(done_size);
            let added = "▒".repeat(added_size);
            let remaining = " ".repeat(remaining_size);
            print!(
                "\r|{done}{added}{remaining}| {:.2} / {:.2}Kb",
                Self::to_kb(self.current + add),
                Self::to_kb(self.target)
            );
            std::io::stdout().flush().unwrap()
        }

        const fn to_kb(n: usize) -> f32 {
            n as f32 / 1024.0
        }
    }

    
    pub struct Args {
        pub address: String,
        pub output: PathBuf,
    }

    pub fn parse_args() -> Args {
        let mut args = env::args().skip(1);
        let mut this = Args {
            address: String::from("127.0.0.1:8080"),
            output: PathBuf::from("data"),
        };

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-a" => {
                    let address = args.next().expect("Expected address after -a");
                    this.address = address;
                }
                "-o" => {
                    let path = args.next().expect("Expected path after -o");
                    this.output = path.into();
                }
                "-h" => {
                    println!("Application that downloads the binary data from the glitchy server");
                    println!("Usage: myftp [-a address] [-o output_path]");
                    std::process::exit(0);
                }
                _ => panic!("Unknown argument: {}", arg),
            }
        }
        this
    }
}
