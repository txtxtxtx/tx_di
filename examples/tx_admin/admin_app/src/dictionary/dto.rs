use admin_domain::dictionary::model::aggregate::{DictType, DictData};
use admin_proto::{DictTypeResponse, DictDataResponse};

/// Convert a domain DictType aggregate into a proto DictTypeResponse
pub fn dict_type_to_response(dt: DictType) -> DictTypeResponse {
    DictTypeResponse {
        id: dt.id,
        name: dt.name,
        dict_type: dt.dict_type,
        status: dt.status,
        remark: dt.remark,
    }
}

/// Convert a domain DictData aggregate into a proto DictDataResponse
pub fn dict_data_to_response(dd: DictData) -> DictDataResponse {
    DictDataResponse {
        id: dd.id,
        sort: dd.sort,
        label: dd.label,
        value: dd.value,
        dict_type: dd.dict_type,
        status: dd.status,
        color_type: dd.color_type,
        css_class: dd.css_class,
        remark: dd.remark,
    }
}
