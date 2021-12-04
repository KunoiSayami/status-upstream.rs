/*
 ** Copyright (C) 2021 KunoiSayami
 **
 ** This file is part of status-upstream.rs and is released under
 ** the AGPL v3 License: https://www.gnu.org/licenses/agpl-3.0.txt
 **
 ** This program is free software: you can redistribute it and/or modify
 ** it under the terms of the GNU Affero General Public License as published by
 ** the Free Software Foundation, either version 3 of the License, or
 ** any later version.
 **
 ** This program is distributed in the hope that it will be useful,
 ** but WITHOUT ANY WARRANTY; without even the implied warranty of
 ** MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 ** GNU Affero General Public License for more details.
 **
 ** You should have received a copy of the GNU Affero General Public License
 ** along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

#[async_trait::async_trait]
trait ServiceChecker {
    async fn check_server(remote_addr: &str, timeout: u64) -> anyhow::Result<bool>;
}


pub mod teamspeak {
    use tokio::net::UdpSocket;
    use tokio::time::Duration;
    use crate::connlib::ServiceChecker;

    const HEAD_DATA: [u8; 34] = hex_literal::hex!("545333494e49543100650000880ef967a500613f9e6966788d480000000000000000");

    pub struct TeamSpeak {}


    #[async_trait::async_trait]
    impl ServiceChecker for TeamSpeak {
        // TODO: Support ipv6
        async fn check_server(remote_addr: &str, timeout: u64) -> anyhow::Result<bool> {
            let socket = UdpSocket::bind("0.0.0.0:0").await?;

            socket.send_to(&HEAD_DATA, remote_addr)
                .await?;

            //socket.set_read_timeout(Duration::from_secs(1));

            let mut buf = [0; 64];
            if let Ok((amt, _src)) = tokio::time::timeout(Duration::from_secs(timeout), socket.recv_from(&mut buf)).await? {
                Ok(amt != 0)
            } else {
                Ok(false)
            }
        }
    }
}


pub mod ssh {

    use tokio::time::Duration;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;
    use crate::connlib::ServiceChecker;

    const HEAD_DATA: [u8; 21] = hex_literal::hex!("5353482d322e302d4f70656e5353485f382e370d0a");

    pub struct SSH {}

    #[async_trait::async_trait]
    impl ServiceChecker for SSH {
        async fn check_server(remote_addr: &str, timeout: u64) -> anyhow::Result<bool> {
            if let Ok(mut socket) = tokio::time::timeout(Duration::from_secs(timeout), TcpStream::connect(remote_addr)).await? {
                if let Ok(_) = tokio::time::timeout(Duration::from_secs(timeout), socket.write_all(&HEAD_DATA)).await? {
                    let mut buff = [0; 64];
                    if let Ok(_) = tokio::time::timeout(Duration::from_secs(timeout), socket.read(&mut buff)).await? {
                        return Ok(String::from_utf8_lossy(&buff).contains("SSH"))
                    }
                }
            }
            Ok(false)
        }
    }

}


pub use teamspeak::TeamSpeak;
pub use ssh::SSH;