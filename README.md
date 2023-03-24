# post.rs

A multi platform email engine for a Microsoft Office 365 backend using IMAP.

## Prerequisites

- **Rust** 1.65 or later  
  Download: [Getting Started](https://www.rust-lang.org/learn/get-started) | Docs: [Rust Book](https://doc.rust-lang.org/book/ch01-01-installation.html)

- **Httpie**  
  Download: [Installation](https://httpie.io/cli) | Docs: [HTTPie Docs](https://httpie.io/docs/cli/usage)

- **jq**  
  Download: [Docs and Installation](https://stedolan.github.io/jq/)

## Roadmap

- [x] Fetch messages (DONE)
- [x] Parse message bodies (DONE)
- [x] Archive (DONE)
- [x] Report Spam (DONE)
- [ ] Own Spam filtering (STARTED)
- [ ] Offline mode

## Setup Azure app for auth

Follow this [Microsoft tutorial](https://docs.microsoft.com/azure/active-directory/develop/quickstart-register-app)

- Register a **Web** application with a **Redirect URI** of `http://localhost:3003/redirect`
- On the left menu select **Overview**. Copy the **Application (client) ID** and add it as the `CLIENT_ID` field to your `.env` file
- On the same menu, select **Certificates & secrets** and add a new client secret. Copy the secret value and add it as the `CLIENT_SECRET` field to your `.env` file
- Now select **API permissions** and add a permission. Select Microsoft Graph and **Delegated permissions**. Add the following permissions:
  - openid
  - profile
  - email
  - offline_access
  - IMAP.AccessAsUser.All
- Add your account name and email address to `ACCOUNT_NAME` and `ACCOUNT_EMAIL`

## Temporary auth method

To authenticate you need to log into your Microsoft account and get an access token. The command below will open your browser and once you login your token will be stored locally:

```sh
cargo run -- auth set
```

Run the `auth get` command and stream it to your httpie request:

```sh
http :3001/api/emails "Authorization: Bearer $(cargo run -q auth get | jq -r .access_code)"
```

You should be able to see a list of emails from your inbox.
