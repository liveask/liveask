![liveask readme header](/assets/readme_header.png)
[![CI](https://github.com/liveask/liveask/actions/workflows/push.yml/badge.svg)](https://github.com/liveask/liveask/actions/workflows/push.yml)  [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)  [![made-with-rust](https://img.shields.io/badge/Made%20with-Rust-1f425f.svg)](https://www.rust-lang.org/)


## TL;DR
Live-Ask.com provides a simple, free, and real-time service for the audience to ask questions before and during panel discussions, conference presentations, meetups, and more. Think of Live-Ask as your one-stop solution for moderating discussions.

* Provides a **simple** event setup for a moderator, with no login necessary
* Is open and **anonymous** for everyone - just share a link
* Allows participants to easily add any question to a discussion
* Operates in **real time** – see what others vote for instantly
* Works on **all platforms** – use it on your phone, tablet, or laptop
* Is **free** to use with premium features available

Live-Ask is a product developed and maintained by [Rustunit.com](https://rustunit.com)

### Socials
[![Instagram](https://img.shields.io/badge/Instagram-%23E4405F.svg?style=for-the-badge&logo=Instagram&logoColor=white)](https://www.instagram.com/liveaskapp/?igshid=OGQ5ZDc2ODk2ZA%3D%3D&utm_source=qr)
[![LinkedIn](https://img.shields.io/badge/linkedin-%230077B5.svg?style=for-the-badge&logo=linkedin&logoColor=white)](https://www.linkedin.com/company/live-ask/)
[![Mastodon](https://img.shields.io/badge/-MASTODON-%232B90D9?style=for-the-badge&logo=mastodon&logoColor=white)](https://mastodon.social/@liveask)
[![TikTok](https://img.shields.io/badge/TikTok-%23000000.svg?style=for-the-badge&logo=TikTok&logoColor=white)](https://www.tiktok.com/@liveaskapp)
[![Twitter](https://img.shields.io/badge/Twitter-%231DA1F2.svg?style=for-the-badge&logo=Twitter&logoColor=white)](https://twitter.com/liveaskapp)
[![ProductHunt](https://img.shields.io/badge/Product%20Hunt-DA552F.svg?style=for-the-badge&logo=Product-Hunt&logoColor=white)](https://www.producthunt.com/products/live-ask)

### Links
* Original Pitch Blogpost for the Project (2018): [Blog Post](https://blog.extrawurst.org/general/webdev/2018/04/02/liveask.html)

### Screenshots
<img src="/assets/desktop_modview.png" height="222" width="250" > <img src="/assets/desktop_partview.png" height="222" width="250" > <img src="/assets/desktop_share.png" height="222" width="250" >

## News & Updates
* **2023** - Opensourcing
* **2022** - Complete Re-write in Rust
* **2018** - Initial Launch of Live-Ask
* **2017** - Work started on Live-Ask

See detailed [changelog](CHANGELOG.md).

## Build & Usages Details
**Requires Three Terminal Tabs/Instances**
### Initial Setup
#### Pre-requisites 
```
How to install Rust
https://www.rust-lang.org/tools/install

How to install Docker
https://docs.docker.com/engine/install

Commands To Run
rustup update
rustup target add wasm32-unknown-unknown
cargo install cargo-make
git clone https://github.com/liveask/liveask.git
cd liveask
```
#### Back End
**First Terminal**
This is required to run up all dependencies for the application 
```
cd backend
make docker-compose
```

**Second Terminal**
This will load up the backend and connect to the dependencies
```
cd backend
make run
```
#### Front End

**Third Terminal**
The to load the frontend
```
cd frontend
make serve
```

### Configuration
To configure the application first copy the default.env to local.env
```
cd backend/env
cp default.env local.env
```
While locally developing to relax CORS policy set `RELAX_CORS` to `"1"` in production leave as `""`
#### Default Configuration
```env
RUST_LOG=warn
RELAX_CORS=""
TINY_URL_TOKEN=""
LA_SENTRY_DSN
LA_ADMIN_PWD_HASH
LA_POSTHOG_KEY
LA_STRIPE_SECRET
LA_STRIPE_HOOK_SECRET
```


## Support

<a href="https://www.producthunt.com/products/live-ask/reviews?utm_source=badge-product_review&utm_medium=badge&utm_souce=badge-live&#0045;ask" target="_blank"><img src="https://api.producthunt.com/widgets/embed-image/v1/product_review.svg?product_id=392197&theme=neutral" alt="Live&#0045;Ask - A&#0032;one&#0045;stop&#0032;solution&#0032;for&#0032;moderating&#0032;discussions&#0032;and&#0032;Q&#0038;As&#0046; | Product Hunt" style="width: 250px; height: 54px;" width="250" height="54" /></a>

### Donations
Wnat to help support the project? Use the following links to help us 💪

[![github](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/extrawurst)

### Attributions

* `viewers.svg`, `likes.svg` and `questions.svg` by [Arafat Uddin](https://thenounproject.com/shalfdesign/)
* `admin.svg` by [LAFS](https://thenounproject.com/LAFS/)

### Contact
(coming soon)


![liveask readme footer](/assets/readme_footer.png)
