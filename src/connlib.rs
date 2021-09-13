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

pub mod teamspeak {
    use tokio::net::UdpSocket;

    const TEST_STRING: [u8; 34] = hex_literal::hex!("545333494e49543100650000880ef967a500613f9e6966788d480000000000000000");

    pub async fn check_server(remote_addr: &str) -> anyhow::Result<bool> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;

        socket.send_to(&TEST_STRING, remote_addr)
            .await
            .expect("Error on send");

        //socket.set_read_timeout(Duration::from_secs(1));

        let mut buf = [0; 2048];
        let (amt, _src) = socket.recv_from(&mut buf).await?;

        Ok(amt == 0)
    }
}