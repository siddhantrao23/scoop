use crate::routes::FormData;

use super::{SubscriberName, SubscriberEmail};

pub struct NewSubscriber {
  pub email: SubscriberEmail,
  pub name: SubscriberName,
}

impl TryFrom<FormData> for NewSubscriber {
  type Error = String;

  fn try_from(form: FormData) -> Result<Self, Self::Error> {
    let name = SubscriberName::parse(form.name)?;
    let email = SubscriberEmail::parse(form.email)?;
    Ok(NewSubscriber { email, name })
  }
}