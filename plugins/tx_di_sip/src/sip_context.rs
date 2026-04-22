use std::sync::Arc;
use rsipstack::EndpointBuilder;
use rsipstack::transaction::Endpoint;
use serde::Deserialize;
use tx_di_core::{tx_comp, BoxFuture, BuildContext, CompInit, RIE};

#[tx_comp(init)]
pub struct SipContext{
    /// 端点
    // #[tx_cst(None)]
    pub end_point: Option<Endpoint>,
    /// 配置
    pub config: Arc<SipConfig>,
}

impl SipContext {


}
impl CompInit for SipContext {
    fn inner_init(&mut self, ctx: &mut BuildContext) ->RIE<()>  {
        self.end_point = Some(EndpointBuilder::new()
            .with_user_agent("tx-di-sip/v0.1.0")
            // .with_option()
            .build());
        Ok(())
    }


    fn init_sort() -> i32 {
        todo!()
    }
}

/// sip 配置
#[derive(Debug, Clone, Deserialize)]
#[tx_comp(conf)]
pub struct SipConfig {
    #[serde(default = "default_ip")]
    pub ip: String,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default)]
    pub ip_version: IpVersion,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IpVersion {
    V4,
    V6,
    Any,
}

impl Default for IpVersion {
    fn default() -> Self {
        IpVersion::Any
    }
}

fn default_ip() -> String {
    "::".to_string()
}

fn default_port() -> u16 {
    5060
}

