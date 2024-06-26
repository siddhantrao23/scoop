use reqwest::Client;
use secrecy::{Secret, ExposeSecret};

use crate::domain::SubscriberEmail;

pub struct EmailClient {
  sender: SubscriberEmail,
  client: Client,
  base_url: String,
  auth_token: Secret<String>,
}

impl EmailClient {
  pub fn new(
    sender: SubscriberEmail,
    base_url: String,
    auth_token: Secret<String>,
    timeout: std::time::Duration,
  ) -> Self {
    let client = Client::builder()
      .timeout(timeout)
      .build()
      .unwrap();
    Self {
      auth_token,
      sender,
      client,
      base_url
    }
  }

  pub async fn send_email(
    &self,
    receiver: &SubscriberEmail,
    subject: &str,
    text_body: &str,
    html_body: &str,
  ) -> Result<(), reqwest::Error> {
    let url = format!("{}/email", self.base_url);
    let request_body = SendEmailRequest {
      from: self.sender.as_ref(),
      to: receiver.as_ref(),
      subject,
      text_body,
      html_body,
    };

    self
      .client
      .post(&url)
      .header("X-Postmark-Server-Token", self.auth_token.expose_secret().clone())
      .json(&request_body)
      .send()
      .await?
      .error_for_status()?;
    Ok(())
  }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
  from: &'a str,
  to: &'a str,
  subject: &'a str,
  text_body: &'a str,
  html_body: &'a str,
}

#[cfg(test)]
mod tests {
  use claims::{assert_ok, assert_err};
  use secrecy::Secret;
  use wiremock::{MockServer, Mock, matchers::{header_exists, header, path, method, any}, ResponseTemplate};
  use fake::{faker::{internet::en::SafeEmail, lorem::en::{Sentence, Paragraph}}, Fake, Faker};
  use crate::{domain::SubscriberEmail, email_client::EmailClient};

  fn subject() -> String {
    Sentence(1 .. 2).fake()
  }
  
  fn body() -> String {
    Sentence(1 .. 10).fake()
  }

  fn email() -> SubscriberEmail {
    SubscriberEmail::parse(SafeEmail().fake()).unwrap()
  }

  fn email_client(base_url: String) -> EmailClient {
    EmailClient::new(
      email(),
      base_url, 
      Secret::new(Faker.fake()),
      std::time::Duration::from_millis(200)
    )
  }

  #[tokio::test]
  async fn send_email_requests_base_url_successfully() {
    let mock_server = MockServer::start().await;
    print!("{}", mock_server.uri());
    let email_client = email_client(mock_server.uri());
    Mock::given(header_exists("X-Postmark-Server-Token"))
      .and(header("Content-Type", "application/json"))
      .and(path("/email"))
      .and(method("POST"))
      .respond_with(ResponseTemplate::new(200))
      .expect(1)
      .mount(&mock_server)
      .await;

    let subscriber_email = email();
    let _ = email_client
      .send_email(&subscriber_email, &subject(), &body(), &body())
      .await;
  }

  #[tokio::test]
  async fn send_email_succeeds_if_200_response() {
    let mock_server = MockServer::start().await;
    let email_client = email_client(mock_server.uri());

    Mock::given(any())
      .respond_with(ResponseTemplate::new(200))
      .expect(1)
      .mount(&mock_server)
      .await;

    let subscriber_email = email();
    let subject: String = Sentence(1 .. 2).fake();
    let body: String = Paragraph(1 .. 10).fake();

    let response = email_client
      .send_email(&subscriber_email, &subject,&body, &body)
      .await;

    assert_ok!(response);
  }
  
  #[tokio::test]
  async fn send_email_fails_if_500_response() {
    let mock_server = MockServer::start().await;
    let email_client = email_client(mock_server.uri());

    Mock::given(any())
      .respond_with(ResponseTemplate::new(500))
      .expect(1)
      .mount(&mock_server)
      .await;

    let subscriber_email = email();
    let subject: String = Sentence(1 .. 2).fake();
    let body: String = Paragraph(1 .. 10).fake();

    let response = email_client
      .send_email(&subscriber_email, &subject, &body, &body)
      .await;

    assert_err!(response);
  }
      
}