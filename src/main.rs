extern crate reqwest;
extern crate select;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use std::io::{BufWriter, Write};

use anyhow::Result;
use reqwest::blocking::ClientBuilder;
use select::document::Document;
use select::predicate::Name;
use structopt::StructOpt;

// If we don't specify a user agent we get a 403 Forbidden from CC
static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(Serialize, Deserialize)]
struct Variant {
    id: i64,
    title: String,
    price: i32,
    name: String,
    inventory_quantity: i8,
}

#[derive(Serialize, Deserialize)]
struct Product {
    options: Option<Vec<Variant>>,
    product_link: String,
}

#[derive(StructOpt, Debug)]
#[structopt(
    about = "A scraper for Community Clothing. Their products are fantastic; their website's search feature is poor."
)]
enum Opt {
    /// Fetch products from the CC website
    Fetch {
        /// Fetch but use the existing links file.
        #[structopt(long, short)]
        use_existing_links: bool,
    },
}
fn main() -> Result<()> {
    let opt = Opt::from_args();
    match opt {
        Opt::Fetch { use_existing_links } => {
            let product_link_file = "data/product_links.txt";
            let base_url = "https://communityclothing.co.uk";
            let menswear_url = format!("{}/collections/menswear/", base_url);
            if !use_existing_links {
                get_products_links(&menswear_url, product_link_file)?;
            } else if use_existing_links && !Path::new(product_link_file).exists() {
                println!("You asked me to use the existing links file but it doesn't exist so I'll get the links now.");
                get_products_links(&menswear_url, product_link_file)?;
            }

            get_products(base_url, product_link_file)?;
        }
    }

    Ok(())
}

fn get_products(url: &str, product_links_file: &str) -> Result<()> {
    let mut csv = csv::Writer::from_path(format!(
        "data/{}_menswear.csv",
        chrono::offset::Local::now()
    ))?;
    if let Ok(lines) = read_lines(product_links_file) {
        for product_link in lines.into_iter().flatten() {
            let product = get_product(url, &product_link)?;
            product
                .options
                .as_ref()
                .unwrap()
                .iter()
                .for_each(|variant| {
                    println!("Writing out variant");
                    csv.write_record(&[
                        &product.product_link,
                        &variant.name,
                        &format!("{}", variant.price),
                        &format!("{}", variant.inventory_quantity),
                    ])
                    .unwrap();
                })
        }
    }
    csv.flush()?;
    Ok(())
}

fn get_product(url: &str, product_link: &str) -> Result<Product> {
    let client = ClientBuilder::new().user_agent(APP_USER_AGENT).build()?;
    let full_url = format!("{}/{}", url, product_link);
    let resp = client.get(full_url).send()?;
    let status = resp.status();

    let mut product: Product = Product {
        product_link: String::from(product_link),
        options: Option::None,
    };
    if status.is_success() {
        let doc = Document::from_read(resp).unwrap();

        // That doesn't tell us about the product though. There's other JSON that'll tell us the sizes.
        doc.find(Name("script")).for_each(|x| {
            let text = x.text();
            if text.contains("KiwiSizing.data") {
                for line in text.split('\n') {
                    if line.contains("variants") {
                        let json_str = &line.trim()[10..].trim_end_matches(',');
                        let options: Vec<Variant> = serde_json::from_str(json_str).unwrap();
                        product.options = Some(options);
                    }
                }
            }
        });
    }
    Ok(product)
}

fn get_products_links(url: &str, product_links_file: &str) -> Result<()> {
    println!("Finding out how many pages there are, and getting the first page of products at the same time.");
    let client = ClientBuilder::new().user_agent(APP_USER_AGENT).build()?;
    let resp = client.get(url).send()?;
    let status = resp.status();

    let mut pages: Vec<i16> = Vec::new();
    let mut product_links: Vec<String> = Vec::new();

    // Get the first page
    if status.is_success() {
        Document::from_read(resp)
            .unwrap()
            .find(Name("a"))
            .filter_map(|n| n.attr("href"))
            .for_each(|x| {
                // Get the total number of pages
                if x.contains("page=") {
                    let index = x.find("page=").unwrap();
                    let page_number = &x[index + 5..].parse::<i16>().unwrap();
                    pages.push(page_number.to_owned());
                } else if x.contains("/products/") {
                    product_links.push(x.to_owned());
                }
            });
    } else {
        println!("Status: {}", status);
        println!("{}", resp.text()?);
    }

    // Get subsequent pages
    pages.sort_unstable();
    let total_pages = pages.last().unwrap();

    // Pages are 1-indexed, and we've already got the first page.
    for page in 2..total_pages.to_owned() + 1 {
        println!("Getting products for page {}", page);
        let resp = client.get(format!("{}?page={}", url, page)).send()?;
        let status = resp.status();
        if status.is_success() {
            Document::from_read(resp)
                .unwrap()
                .find(Name("a"))
                .filter_map(|n| n.attr("href"))
                .for_each(|x| {
                    if x.contains("/products/") {
                        product_links.push(x.to_owned());
                    }
                });
        } else {
            println!("Status: {}", status);
            println!("{}", resp.text()?);
        }
    }

    product_links.sort();
    product_links.dedup();

    let f = File::create(product_links_file).expect("Unable to create file");
    let mut f = BufWriter::new(f);
    for product_link in product_links.iter() {
        f.write_all(format!("{}\n", product_link).as_bytes())
            .expect("Unable to write data");
    }

    Ok(())
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
