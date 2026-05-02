pub mod cache;
pub mod preview;
pub mod session;
pub mod timing;

use crate::app::media::error::{MediaCommandError, MediaResult};

fn command_result<T>(result: MediaResult<T>) -> Result<T, MediaCommandError> {
    result.map_err(Into::into)
}
