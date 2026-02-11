use slint::ComponentHandle; 

slint::slint! {
    import { LineEdit, Button, VerticalBox, HorizontalBox } from "std-widgets.slint";

    export component AppWindow inherits Window {
        title: "Albedo Browser";
        min-width: 800px;
        min-height: 600px;
        background: #111111; 

        callback navigate(string); 

        VerticalLayout {
            padding: 0px;
            
            Rectangle {
                background: #1a1a1a;
                height: 40px;
                border-bottom-width: 1px;
                border-color: #333333;

                HorizontalLayout {
                    padding: 5px;
                    spacing: 10px;

                    Text {
                        text: "ALBEDO://";
                        color: #00ff00; 
                        font-family: "Monospace";
                        vertical-alignment: center;
                    }

                    url_input := LineEdit {
                        placeholder-text: "Digite uma URL ou comando...";
                        font-size: 14px;
                        height: 30px;
                        accepted => { root.navigate(self.text); }
                    }

                    Button {
                        text: "GO";
                        width: 40px;
                        height: 30px;
                        clicked => { root.navigate(url_input.text); }
                    }
                }
            }

            Rectangle {
                background: #000000;
                
                VerticalLayout {
                    alignment: center;
                    Text {
                        text: "CORE: IDLE";
                        color: #444444;
                        font-size: 20px;
                        horizontal-alignment: center;
                    }
                    Text {
                        text: "Waiting for signal...";
                        color: #222222;
                        font-size: 12px;
                        horizontal-alignment: center;
                    }
                }
            }
            
            Rectangle {
                height: 20px;
                background: #0a0a0a;
                Text {
                    x: 5px;
                    text: "RAM: LOW | CPU: 1% | MODE: BATATA";
                    color: #555555;
                    font-size: 10px;
                    vertical-alignment: center;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    let ui_handle = ui.as_weak();

    ui.on_navigate(move |url| {
        let _ui = ui_handle.unwrap();
        println!("Navegando para: {}", url);
        
        if url == "about:albedo" {
            println!("Versão Alpha 0.1");
        }
    });

    ui.run()
}