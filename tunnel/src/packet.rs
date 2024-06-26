use etherparse::{checksum, Ipv4HeaderSlice, PacketBuilder, TcpHeaderSlice};

const IPV4_HEADER_LEN: usize = 20;

pub fn create_handshake_packet(ip_addr: &[u8; 4]) -> Vec<u8> {
    let builder = PacketBuilder::ipv4(ip_addr.clone(), [0, 0, 0, 0], 10).udp(1, 1);
    let payload = [1];
    let mut result = Vec::<u8>::with_capacity(builder.size(payload.len()));
    builder.write(&mut result, &payload).unwrap();
    return result;
}

pub fn is_handshake_packet(buf: &[u8]) -> bool {
    let slice = Ipv4HeaderSlice::from_slice(&buf);
    if slice.is_err() {
        println!("{:?}", slice.err().unwrap());
        return false;
    }
    slice.unwrap().destination_addr().is_unspecified()
}

pub fn change_address_and_port(buf: &mut [u8], addr: &[u8], port: u16, is_source: bool) -> u16 {
    let mut offset = 0;
    if !is_source {
        offset = 4;
    }
    buf[12 + offset] = addr[0];
    buf[13 + offset] = addr[1];
    buf[14 + offset] = addr[2];
    buf[15 + offset] = addr[3];
    let mut ip_header_length = (buf[0] & 15) as usize;
    ip_header_length *= 4;
    set_header_checksum(&mut buf[..ip_header_length]);
    let result_port;
    let port = port.to_be_bytes();
    if is_source {
        result_port = 0;
        buf[ip_header_length] = port[0];
        buf[ip_header_length + 1] = port[1];
    } else {
        result_port = u16::from_be_bytes([buf[ip_header_length+2], buf[ip_header_length + 3]]);
        buf[ip_header_length + 2] = port[0];
        buf[ip_header_length + 3] = port[1];
    }
    set_tcp_checksum(buf, ip_header_length);
    result_port
}

/// Due to the length change done by the encryption/decryption process, a new header checksum has
/// to be calculated. This prevents the kernel from dropping our encrypted/decrypted packets.
/// This also sets the checksum in the packet bytes indexes.
pub fn set_header_checksum(buf: &mut [u8]) {
    let mut csum = checksum::Sum16BitWords::new();
    for x in (0..10).step_by(2) {
        csum = csum.add_2bytes([buf[x], buf[x + 1]]);
    }
    csum = csum.add_4bytes([buf[12], buf[13], buf[14], buf[15]]);
    csum = csum.add_4bytes([buf[16], buf[17], buf[18], buf[19]]);
    if buf.len() > IPV4_HEADER_LEN {
        csum = csum.add_slice(&buf[IPV4_HEADER_LEN..]);
    }
    let sum = csum.ones_complement().to_be().to_be_bytes();
    buf[10] = sum[0];
    buf[11] = sum[1];
}

pub fn get_version(buf: &[u8]) -> u8 {
    buf[0] >> 4
}

pub fn set_tcp_checksum(buf: &mut [u8], ip_header_length: usize) {
    let tcp_packet = &buf[ip_header_length..];
    let tcp_header = TcpHeaderSlice::from_slice(tcp_packet).unwrap();
    let header_len = ip_header_length + ((tcp_packet[12] & 0xf0) >> 2) as usize;
    let checksum = tcp_header
        .calc_checksum_ipv4_raw(
            [buf[12], buf[13], buf[14], buf[15]],
            [buf[16], buf[17], buf[18], buf[19]],
            &buf[header_len..],
        )
        .unwrap();
    let sum = checksum.to_be_bytes();
    buf[ip_header_length + 16] = sum[0];
    buf[ip_header_length + 17] = sum[1];
}
