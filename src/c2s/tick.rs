use crate::state;
use azalea::Client;
use luanti_protocol::LuantiConnection;

pub async fn tick(
    _luanti_conn: &mut LuantiConnection,
    _mc_client: &mut Client,
    _proxy_state: &mut state::ProxyState,
) {
    // nothing to do here yet!
}
