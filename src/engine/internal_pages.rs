pub fn get_internal_page(url: &str) -> Option<(String, String)> {
    if !url.starts_with("albedo://") {
        return None;
    }

    let title = format!("Albedo - {}", &url[9..]);
    let content = match url {
        "albedo://about" => "<h1>About Albedo</h1><p style='color: blue;'>The pure Rust browser.</p>".to_string(),
        "albedo://engine" => "<h1>ACE v0.1</h1><p style='color: red;'>Running natively.</p>".to_string(),
        "albedo://flex" => "<h1>Flexbox Demo</h1><div style='display: flex; flex-direction: row; justify-content: space-between;'><div style='background-color: red; width: 50px; height: 50px;'></div><div style='background-color: blue; width: 50px; height: 50px;'></div></div>".to_string(),
        "albedo://images" => "<h1>Images Demo</h1><p>Displaying images via ACE:</p><img src='https://www.rust-lang.org/static/images/rust-logo-blk.svg' width='150' height='150'><p>Local image:</p><img src='assets/icon.png' width='50' height='50'>".to_string(),
        "albedo://css" => "
            <style>
                .title { color: purple; font-size: 40px; }
                #special { color: red; background-color: yellow; }
                p { color: green; }
                .box { display: flex; flex-direction: row; justify-content: space-around; background-color: #eee; }
                .item { color: white; background-color: blue; font-size: 20px; }
            </style>
            <h1 class='title'>CSS Engine Demo</h1>
            <p id='special'>This is a special ID-styled paragraph.</p>
            <p>This is a normal paragraph styled by tag selector.</p>
            <div class='box'>
                <div class='item'>Item 1</div>
                <div class='item'>Item 2</div>
                <div class='item'>Item 3</div>
            </div>
        ".to_string(),
        "albedo://links" => "<h1>Links Demo</h1>
            <p>Click the link below:</p>
            <a href='albedo://flex'>Go to Flexbox Demo</a>
            <br/>
            <div style='background-color: #eee; padding: 10px;'>
                <a href='albedo://about'>Go to About</a>
            </div>".to_string(),
        "albedo://scroll" => "<h1>Scroll Demo</h1>
            <p>This page should scroll.</p>
            <div style='height: 200px; background-color: red;'>Item 1</div>
            <div style='height: 200px; background-color: blue;'>Item 2</div>
            <div style='height: 200px; background-color: green;'>Item 3</div>
            <div style='height: 200px; background-color: yellow;'>Item 4</div>
            <div style='height: 200px; background-color: purple;'>Item 5</div>
            <p>End of page.</p>".to_string(),
        "albedo://start" => "<h1>Welcome to Albedo</h1><p>The smoothest browser.</p>".to_string(), // Ensure start page is covered
        _ => "<h1>404</h1><p>Internal page not found.</p>".to_string(),
    };

    Some((title, content))
}
