# community_clothing_scraper

Community Clothing make fantastic clothes but their website doesn't allow faceted searching. I.e. I can't search for stock in my size. And because of the awesome way the business is structured (read about it on their site) they don't always have stock. Yes they have email alerts but sometimes I just want to browse what's available to me and not set alerts on ten different pages for different coloured t-shirts or whatever. Right now I go to their site and I'm almost always dissapointed.

This is a scraper written in rust that pulls down all menswear. I feel so lame that I've only done it for menswear. I'd like to add an option for womenswear. Menswear wouldn't be the default (thank you [Caroline Criado Perez](https://carolinecriadoperez.com/)).

## Installation

I've not publised this as a crate. Clone the repo and execute `cargo run` to see the options, or just run `cargo run fetch` to kick it off. It'll produce a CSV you can load into a spreadsheet program and start filtering.