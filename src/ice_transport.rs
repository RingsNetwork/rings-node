use std::unimplemented;


pub struct IceTransport {
    pub candidate: Option<String>
}

impl IceTransport {
    pub async fn new() -> Self {
        unimplemented!();
    }

    pub async fn candiate(&self) -> Option<String> {
        unimplemented!();
    }

}
