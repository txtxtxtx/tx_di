use admin_domain::config::model::aggregate::Config;
use admin_proto::ConfigResponse;

/// Convert a domain Config aggregate into a proto ConfigResponse
pub fn config_to_response(c: Config) -> ConfigResponse {
    ConfigResponse {
        id: c.id,
        category: c.category,
        config_type: c.config_type,
        name: c.name,
        config_key: c.config_key,
        value: c.value,
        visible: c.visible,
        remark: c.remark,
    }
}
