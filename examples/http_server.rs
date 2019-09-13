use {
    std::io,

    minitokio::{
        self,
        Runtime,
        TcpListener,
        TcpStream,
    },
};

/// A reusable buffer for reading several pipelined requests.
/// `pos` points at the start of the first unprocessed request,
/// `end` points at the end of the data read from the socket.
struct RequestBuf {
    buf: Box<[u8]>,
    pos: usize,
    end: usize,
}

impl RequestBuf {
    fn new() -> RequestBuf {
        RequestBuf {
            buf: Box::new([0; 4096]),
            pos: 0,
            end: 0,
        }
    }

    fn advance(&mut self, nread: usize) -> io::Result<()> {
        if nread == 0 && self.pos != self.end {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "incomplete request"))
        }

        self.end += nread;
        Ok(())
    }

    /// Make room for the next read invocation if we filled the buffer to the end.
    fn rewind(&mut self) -> io::Result<()> {
        if self.pos == self.buf.len() {
            // we've read the full request and it ended exactly at the end of the buffer.
            self.pos = 0;
            self.end = 0;
        } else if self.end == self.buf.len() {
            if self.pos == 0 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "request too big"));
            } else {
                // we've read part of the request and need to copy it to the beginning of the buffer to read the rest
                self.buf.rotate_left(self.pos);
                self.end = self.end - self.pos;
                self.pos = 0;
            }
        }
        Ok(())
    }
}

impl AsMut<[u8]> for RequestBuf {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.buf[self.end..]
    }
}

/// Try to parse and process the request and to write out the response.
/// Returns Ok(true) if the request was actually processed.
fn process_request(in_buf: &mut RequestBuf, peer_addr: &str, out_buf: &mut impl io::Write)
                       -> Result<bool, io::Error> {
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut parsed = httparse::Request::new(&mut headers);
    let res = parsed.parse(&in_buf.buf[in_buf.pos..in_buf.end]);
    match res {
        Ok(httparse::Status::Complete(nparsed)) => {
            in_buf.pos += nparsed;

            // eprintln!("processing request: {} {}", parsed.method.unwrap(), parsed.path.unwrap());

            // // simulate cpu-intensive work
            // std::thread::sleep(std::time::Duration::from_millis(100));

            let now = chrono::Local::now().format("%a, %d %b %Y %T %Z");

            use std::fmt::Write;
            let mut payload = String::new();
            write!(payload, "<html>\n\
                             <head>\n\
                             <title>Test page</title>\n\
                             </head>\n\
                             <body>\n\
                             <p>Hello, your address is {}, current time is {}.</p>\n\
                             </body>\n\
                             </html>",
                   peer_addr, now).unwrap();

            write!(out_buf, "HTTP/1.1 200 OK\r\n\
                             Server: MyHTTP\r\n\
                             Content-Type: text/html\r\n\
                             Content-Length: {}\r\n\
                             Date: {}\r\n\
                             \r\n\
                             {}\r\n",
                   payload.len() + 2, now, payload).unwrap();

            Ok(true)
        }

        Ok(httparse::Status::Partial) => Ok(false),

        Err(_) => Err(io::Error::new(io::ErrorKind::InvalidData, "could not parse request")),
    }
}

async fn process_conn(mut conn: TcpStream, peer_addr: String)
                      -> Result<(), Box<dyn std::error::Error>> {
    let mut in_buf = RequestBuf::new();
    let mut resp = Vec::new();

    loop {
        in_buf.rewind()?;
        let nread = conn.read(in_buf.as_mut()).await?;

        in_buf.advance(nread)?;
        if nread == 0 {
            eprintln!("client closed the connection from {}", peer_addr);
            return Ok(());
        }

        resp.clear();
        if process_request(&mut in_buf, &peer_addr, &mut resp)? {
            let mut pos = 0;
            while pos < resp.len() {
                pos += conn.write(&resp[pos..]).await?;
            }
        }
    }
}

async fn server_main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = std::env::args().nth(1).unwrap_or("127.0.0.1:8000".to_string());
    let mut listener = TcpListener::bind(&addr.parse()?)?;
    println!("listening on {}", addr);

    loop {
        let (conn, peer_addr) = listener.accept().await?;
        minitokio::spawn(async move {
            if let Err(e) = process_conn(conn, peer_addr.to_string()).await {
                eprintln!("error while processing connection from {}: {}", peer_addr, e);
            }
        });
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut runtime = Runtime::new(3)?;
    runtime.run(server_main())
}
