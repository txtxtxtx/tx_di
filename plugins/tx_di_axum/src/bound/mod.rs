use std::any::Any;
use std::ops::Deref;
use std::sync::Arc;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use tx_di_core::{App, BuildContext, ComponentDescriptor, IE, RIE};
use crate::e::WebErr;

pub struct AppStatus {
    pub app: Arc<App>,
}

pub trait RequestPartsExt {
    fn app_status(&self) -> &AppStatus;
    /// 从 DI 容器中取出已缓存的单例组件（只读，无需 mut context）
    fn get_comp<T: ComponentDescriptor>(&self) -> RIE<Arc<T>>;
}

impl RequestPartsExt for Parts {
    fn app_status(&self) -> &AppStatus {
        self.extensions.get::<AppStatus>()
            .expect("AppStatus not found in request extensions; 请确认已通过 layer 注入 AppStatus")
    }

    fn get_comp<T: ComponentDescriptor>(&self) -> RIE<Arc<T>> {
        self.app_status()
            .app
            .try_inject::<T>()
            .ok_or_else(|| IE::Other(format!(
                "组件 {} 未在 DI 容器中找到，请确认已用 #[tx_comp] 注解并完成初始化",
                std::any::type_name::<T>()
            )))
    }
}
pub struct DiComp<T: ComponentDescriptor>(pub Arc<T>);
impl<T: ComponentDescriptor> Deref for DiComp<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, S> FromRequestParts<S> for DiComp<T>
where
    T: ComponentDescriptor,
    S: Send + Sync,
{
    type Rejection = WebErr;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let comp = parts.get_comp::<T>().map_err(WebErr::IE)?;
        Ok(DiComp(comp))
    }
}