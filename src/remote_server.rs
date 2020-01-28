use std::sync::mpsc;

struct Credentials {
    username: String,
    password: String,
}

struct Server {
    url: String,
    credentials: Option<Credentials>,

    connected: bool,
    sync_thread: Option<std::thread::JoinHandle<()>>,
}

enum Error {
    BadUrl,
}

enum ServerRequest {
    Connect,
}

impl Server {
    fn new(url: String, credentials: Option<Credentials>) -> Result<Self, Error> {
        // match url.parse() {
        //     Ok(_) => (),
        //     Err(_) => return Err(Error::BadUrl),
        // };
        Ok(Self {
            url,
            credentials,
            connected: false,
            sync_thread: None,
        })
    }

    async fn start(&mut self, sender: mpsc::Sender<String>, receiver: mpsc::Receiver<ServerRequest>) {
        let client = ruma_client::Client::new(self.url.parse().unwrap(), None);

        // Start syncing thread

        // Start user watching loop
        let mut session = None;
        loop {
            let ev = match receiver.recv() {
                Ok(ev) => ev,
                Err(_) => break,
            };

            match ev {
                ServerRequest::Connect => {
                    let session_res = match &self.credentials {
                        None => client.register_guest().await,
                        Some(c) => client
                            .log_in(c.username.clone(), c.password.clone() ,None)
                            .await
                    };
                    session = match session_res {
                        Ok(s) => Some(s),
                        Err(e) => {
                            todo!("Send error events {:#?}", e);
                            None
                        },
                    }
                },
            }
        }
    }
}
