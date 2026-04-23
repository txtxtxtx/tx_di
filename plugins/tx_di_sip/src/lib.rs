mod sip_context;
mod config;
mod comp;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use tx_di_core::BuildContext;
    use super::*;

    #[test]
    fn it_works() {
        BuildContext::default();
    }
}
