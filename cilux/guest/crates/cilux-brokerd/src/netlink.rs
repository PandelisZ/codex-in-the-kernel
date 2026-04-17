#[cfg(target_os = "linux")]
mod imp {
    use anyhow::{Context, Result, bail};
    use cilux_common::FAMILY_VERSION;
    use std::io;
    use std::mem::size_of;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

    const NETLINK_GENERIC: libc::c_int = 16;
    const GENL_ID_CTRL: u16 = 0x10;
    const CTRL_CMD_GETFAMILY: u8 = 3;
    const CTRL_ATTR_FAMILY_ID: u16 = 1;
    const CTRL_ATTR_FAMILY_NAME: u16 = 2;

    const NLM_F_REQUEST: u16 = 0x01;
    const GENL_CTRL_VERSION: u8 = 2;

    const CILUX_CMD_PING: u8 = 1;
    const CILUX_CMD_GET_CAPS: u8 = 2;
    const CILUX_CMD_GET_STATE: u8 = 3;
    const CILUX_CMD_SET_TRACE_MASK: u8 = 4;
    const CILUX_CMD_CLEAR_EVENTS: u8 = 5;

    const CILUX_A_TRACE_MASK: u16 = 1;
    const CILUX_A_SUPPORTED_MASK: u16 = 2;
    const CILUX_A_DROP_COUNT: u16 = 3;
    const CILUX_A_EVENT_COUNT: u16 = 4;
    const CILUX_A_RING_CAPACITY: u16 = 5;
    const CILUX_A_STATUS: u16 = 6;

    const NLMSG_ERROR: u16 = 0x02;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct SockAddrNl {
        nl_family: u16,
        nl_pad: u16,
        nl_pid: u32,
        nl_groups: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NlMsgHdr {
        nlmsg_len: u32,
        nlmsg_type: u16,
        nlmsg_flags: u16,
        nlmsg_seq: u32,
        nlmsg_pid: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct GenlMsgHdr {
        cmd: u8,
        version: u8,
        reserved: u16,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NlAttr {
        nla_len: u16,
        nla_type: u16,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct KernelCaps {
        pub supported_mask: u32,
        pub ring_capacity: u32,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct KernelState {
        pub trace_mask: u32,
        pub supported_mask: u32,
        pub drop_count: u32,
        pub event_count: u32,
        pub ring_capacity: u32,
    }

    pub fn ping() -> Result<()> {
        let family_id = resolve_family_id()?;
        let attrs = transact(family_id, CILUX_CMD_PING, FAMILY_VERSION, &[])?;
        let status = parse_u32_attr(&attrs, CILUX_A_STATUS).context("status missing from ping")?;
        if status != 1 {
            bail!("unexpected ping status {status}");
        }
        Ok(())
    }

    pub fn get_caps() -> Result<KernelCaps> {
        let family_id = resolve_family_id()?;
        let attrs = transact(family_id, CILUX_CMD_GET_CAPS, FAMILY_VERSION, &[])?;
        Ok(KernelCaps {
            supported_mask: parse_u32_attr(&attrs, CILUX_A_SUPPORTED_MASK)
                .context("supported mask missing")?,
            ring_capacity: parse_u32_attr(&attrs, CILUX_A_RING_CAPACITY)
                .context("ring capacity missing")?,
        })
    }

    pub fn get_state() -> Result<KernelState> {
        let family_id = resolve_family_id()?;
        let attrs = transact(family_id, CILUX_CMD_GET_STATE, FAMILY_VERSION, &[])?;
        Ok(KernelState {
            trace_mask: parse_u32_attr(&attrs, CILUX_A_TRACE_MASK).context("trace mask missing")?,
            supported_mask: parse_u32_attr(&attrs, CILUX_A_SUPPORTED_MASK)
                .context("supported mask missing")?,
            drop_count: parse_u32_attr(&attrs, CILUX_A_DROP_COUNT).context("drop count missing")?,
            event_count: parse_u32_attr(&attrs, CILUX_A_EVENT_COUNT)
                .context("event count missing")?,
            ring_capacity: parse_u32_attr(&attrs, CILUX_A_RING_CAPACITY)
                .context("ring capacity missing")?,
        })
    }

    pub fn set_trace_mask(trace_mask: u32) -> Result<u32> {
        let family_id = resolve_family_id()?;
        let attrs = transact(
            family_id,
            CILUX_CMD_SET_TRACE_MASK,
            FAMILY_VERSION,
            &encode_u32_attr(CILUX_A_TRACE_MASK, trace_mask),
        )?;
        parse_u32_attr(&attrs, CILUX_A_TRACE_MASK).context("trace mask missing from response")
    }

    pub fn clear_events() -> Result<u32> {
        let family_id = resolve_family_id()?;
        let attrs = transact(family_id, CILUX_CMD_CLEAR_EVENTS, FAMILY_VERSION, &[])?;
        parse_u32_attr(&attrs, CILUX_A_EVENT_COUNT).context("event count missing from clear reply")
    }

    fn resolve_family_id() -> Result<u16> {
        let attrs = transact(
            GENL_ID_CTRL,
            CTRL_CMD_GETFAMILY,
            GENL_CTRL_VERSION,
            &encode_string_attr(CTRL_ATTR_FAMILY_NAME, "cilux"),
        )?;
        parse_u16_attr(&attrs, CTRL_ATTR_FAMILY_ID).context("family id missing from ctrl reply")
    }

    fn transact(message_type: u16, cmd: u8, version: u8, attr_bytes: &[u8]) -> Result<Vec<u8>> {
        let fd = unsafe {
            libc::socket(
                libc::AF_NETLINK,
                libc::SOCK_RAW | libc::SOCK_CLOEXEC,
                NETLINK_GENERIC,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error()).context("create netlink socket");
        }

        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        let addr = SockAddrNl {
            nl_family: libc::AF_NETLINK as u16,
            nl_pad: 0,
            nl_pid: 0,
            nl_groups: 0,
        };

        let bind_ret = unsafe {
            libc::bind(
                fd.as_raw_fd(),
                &addr as *const SockAddrNl as *const libc::sockaddr,
                size_of::<SockAddrNl>() as u32,
            )
        };
        if bind_ret < 0 {
            return Err(io::Error::last_os_error()).context("bind netlink socket");
        }

        let mut payload = Vec::new();
        payload.extend_from_slice(
            &GenlMsgHdr {
                cmd,
                version,
                reserved: 0,
            }
            .as_bytes(),
        );
        payload.extend_from_slice(attr_bytes);

        let header = NlMsgHdr {
            nlmsg_len: (size_of::<NlMsgHdr>() + payload.len()) as u32,
            nlmsg_type: message_type,
            nlmsg_flags: NLM_F_REQUEST,
            nlmsg_seq: 1,
            nlmsg_pid: 0,
        };

        let mut wire = Vec::new();
        wire.extend_from_slice(header.as_bytes());
        wire.extend_from_slice(&payload);

        let send_ret = unsafe {
            libc::send(
                fd.as_raw_fd(),
                wire.as_ptr() as *const libc::c_void,
                wire.len(),
                0,
            )
        };
        if send_ret < 0 {
            return Err(io::Error::last_os_error()).context("send netlink request");
        }

        let mut response = vec![0u8; 4096];
        let recv_ret = unsafe {
            libc::recv(
                fd.as_raw_fd(),
                response.as_mut_ptr() as *mut libc::c_void,
                response.len(),
                0,
            )
        };
        if recv_ret < 0 {
            return Err(io::Error::last_os_error()).context("receive netlink response");
        }
        response.truncate(recv_ret as usize);

        parse_reply(&response)
    }

    fn parse_reply(response: &[u8]) -> Result<Vec<u8>> {
        if response.len() < size_of::<NlMsgHdr>() {
            bail!("short netlink response");
        }

        let header = unsafe { &*(response.as_ptr() as *const NlMsgHdr) };
        if header.nlmsg_type == NLMSG_ERROR {
            bail!("kernel returned NLMSG_ERROR");
        }

        let genl_offset = size_of::<NlMsgHdr>();
        let attr_offset = genl_offset + size_of::<GenlMsgHdr>();
        if response.len() < attr_offset {
            bail!("short generic netlink response");
        }

        Ok(response[attr_offset..].to_vec())
    }

    fn parse_u32_attr(attrs: &[u8], attr_type: u16) -> Option<u32> {
        find_attr(attrs, attr_type).and_then(|bytes| {
            if bytes.len() < 4 {
                return None;
            }
            Some(u32::from_ne_bytes(bytes[..4].try_into().ok()?))
        })
    }

    fn parse_u16_attr(attrs: &[u8], attr_type: u16) -> Option<u16> {
        find_attr(attrs, attr_type).and_then(|bytes| {
            if bytes.len() < 2 {
                return None;
            }
            Some(u16::from_ne_bytes(bytes[..2].try_into().ok()?))
        })
    }

    fn find_attr(attrs: &[u8], attr_type: u16) -> Option<&[u8]> {
        let mut offset = 0usize;
        while offset + size_of::<NlAttr>() <= attrs.len() {
            let attr = unsafe { &*(attrs[offset..].as_ptr() as *const NlAttr) };
            let len = attr.nla_len as usize;
            if len < size_of::<NlAttr>() || offset + len > attrs.len() {
                break;
            }

            let data_start = offset + size_of::<NlAttr>();
            let data_end = offset + len;
            if attr.nla_type == attr_type {
                return Some(&attrs[data_start..data_end]);
            }

            offset += align4(len);
        }

        None
    }

    fn encode_u32_attr(attr_type: u16, value: u32) -> Vec<u8> {
        encode_attr(attr_type, &value.to_ne_bytes())
    }

    fn encode_string_attr(attr_type: u16, value: &str) -> Vec<u8> {
        let mut bytes = value.as_bytes().to_vec();
        bytes.push(0);
        encode_attr(attr_type, &bytes)
    }

    fn encode_attr(attr_type: u16, value: &[u8]) -> Vec<u8> {
        let len = size_of::<NlAttr>() + value.len();
        let mut out = Vec::with_capacity(align4(len));
        out.extend_from_slice(
            &NlAttr {
                nla_len: len as u16,
                nla_type: attr_type,
            }
            .as_bytes(),
        );
        out.extend_from_slice(value);
        while out.len() % 4 != 0 {
            out.push(0);
        }
        out
    }

    fn align4(len: usize) -> usize {
        (len + 3) & !3
    }

    trait AsBytes {
        fn as_bytes(&self) -> &[u8];
    }

    impl<T> AsBytes for T {
        fn as_bytes(&self) -> &[u8] {
            unsafe { std::slice::from_raw_parts(self as *const T as *const u8, size_of::<T>()) }
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod imp {
    use anyhow::{Result, bail};

    #[derive(Debug, Clone, Copy)]
    pub struct KernelCaps {
        pub supported_mask: u32,
        pub ring_capacity: u32,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct KernelState {
        pub trace_mask: u32,
        pub supported_mask: u32,
        pub drop_count: u32,
        pub event_count: u32,
        pub ring_capacity: u32,
    }

    fn unsupported<T>() -> Result<T> {
        bail!("generic netlink is only available when compiling for Linux")
    }

    pub fn ping() -> Result<()> {
        unsupported()
    }

    pub fn get_caps() -> Result<KernelCaps> {
        unsupported()
    }

    pub fn get_state() -> Result<KernelState> {
        unsupported()
    }

    pub fn set_trace_mask(_trace_mask: u32) -> Result<u32> {
        unsupported()
    }

    pub fn clear_events() -> Result<u32> {
        unsupported()
    }
}

pub use imp::*;
