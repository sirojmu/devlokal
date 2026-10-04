//! Pembuat konfigurasi nginx, php.ini, dan my.ini.

use crate::home::{Home, Settings};
use crate::projects::Project;
use std::path::Path;

/// Path dengan garis miring biasa agar valid di konfigurasi Windows maupun Unix.
pub fn fwd(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

const FASTCGI: &str = r#"      fastcgi_param QUERY_STRING $query_string;
      fastcgi_param REQUEST_METHOD $request_method;
      fastcgi_param CONTENT_TYPE $content_type;
      fastcgi_param CONTENT_LENGTH $content_length;
      fastcgi_param SCRIPT_NAME $fastcgi_script_name;
      fastcgi_param REQUEST_URI $request_uri;
      fastcgi_param DOCUMENT_URI $document_uri;
      fastcgi_param DOCUMENT_ROOT $document_root;
      fastcgi_param SERVER_PROTOCOL $server_protocol;
      fastcgi_param REQUEST_SCHEME $scheme;
      fastcgi_param GATEWAY_INTERFACE CGI/1.1;
      fastcgi_param SERVER_SOFTWARE devlokal;
      fastcgi_param REMOTE_ADDR $remote_addr;
      fastcgi_param REMOTE_PORT $remote_port;
      fastcgi_param SERVER_ADDR $server_addr;
      fastcgi_param SERVER_PORT $server_port;
      fastcgi_param SERVER_NAME $server_name;
      fastcgi_param REDIRECT_STATUS 200;
      fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;
"#;

const MIME: &str = r#"  types {
    text/html html htm;
    text/css css;
    text/plain txt;
    application/javascript js mjs;
    application/json json;
    application/xml xml;
    image/png png;
    image/jpeg jpg jpeg;
    image/gif gif;
    image/svg+xml svg;
    image/webp webp;
    image/x-icon ico;
    font/woff woff;
    font/woff2 woff2;
    application/pdf pdf;
    application/zip zip;
  }
  default_type application/octet-stream;
"#;

pub fn render_nginx(home: &Home, s: &Settings, projects: &[Project]) -> String {
    let daemon = if cfg!(windows) { "" } else { "daemon off;\n" };
    let mut out = format!(
        "# Dihasilkan oleh devlokal — jangan diedit manual, akan ditimpa saat `devlokal sync`.\n\
         worker_processes 1;\n{daemon}pid \"{pid}\";\n\
         events {{ worker_connections 256; }}\n\
         http {{\n{MIME}  sendfile off;\n  client_max_body_size 64m;\n  access_log off;\n\n",
        pid = fwd(&home.run().join("nginx.pid")),
    );
    out.push_str(&format!(
        "  server {{\n    listen 127.0.0.1:{port} default_server;\n    server_name _;\n    default_type text/plain;\n    return 200 \"devlokal aktif. Buat proyek: devlokal new <nama>\\n\";\n  }}\n",
        port = s.http_port
    ));
    for p in projects {
        out.push_str(&format!(
            "\n  server {{\n    listen 127.0.0.1:{port};\n    server_name {domain};\n    root \"{root}\";\n    index index.php index.html;\n\n    location / {{\n      try_files $uri $uri/ /index.php?$query_string;\n    }}\n\n    location ~ \\.php$ {{\n      try_files $uri =404;\n{FASTCGI}      fastcgi_pass 127.0.0.1:{php};\n    }}\n\n    location ~ /\\.(?!well-known) {{ deny all; }}\n  }}\n",
            port = s.http_port,
            domain = p.domain,
            root = fwd(&p.docroot),
            php = s.php_port,
        ));
    }
    out.push_str("}\n");
    out
}

pub fn render_php_ini(php_dir: &Path) -> String {
    let mut ini = String::from(
        "; Dihasilkan oleh devlokal.\n[PHP]\n\
         display_errors = On\nerror_reporting = E_ALL\n\
         memory_limit = 256M\nupload_max_filesize = 64M\npost_max_size = 64M\n\
         date.timezone = Asia/Jakarta\n",
    );
    if cfg!(windows) {
        ini.push_str(&format!(
            "extension_dir = \"{}\"\n",
            fwd(&php_dir.join("ext"))
        ));
        for e in [
            "curl",
            "fileinfo",
            "gd",
            "mbstring",
            "mysqli",
            "openssl",
            "pdo_mysql",
            "pdo_sqlite",
            "zip",
        ] {
            ini.push_str(&format!("extension={e}\n"));
        }
    }
    ini
}

pub fn render_my_ini(home: &Home, s: &Settings, basedir: &Path) -> String {
    format!(
        "# Dihasilkan oleh devlokal.\n[mysqld]\nbasedir=\"{base}\"\ndatadir=\"{data}\"\nport={port}\nbind-address=127.0.0.1\n\
         character-set-server=utf8mb4\ncollation-server=utf8mb4_general_ci\n\n[client]\nport={port}\n",
        base = fwd(basedir),
        data = fwd(&home.data().join("mariadb")),
        port = s.db_port,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn nginx_has_vhost_and_php_upstream() {
        let home = Home::new("/tmp/dl");
        let s = Settings::default();
        let ps = vec![Project {
            name: "blog".into(),
            domain: "blog.test".into(),
            docroot: PathBuf::from("/tmp/dl/www/blog/public"),
        }];
        let c = render_nginx(&home, &s, &ps);
        assert!(c.contains("server_name blog.test;"));
        assert!(c.contains("root \"/tmp/dl/www/blog/public\";"));
        assert!(c.contains("fastcgi_pass 127.0.0.1:9000;"));
        assert_eq!(c.matches('{').count(), c.matches('}').count());
    }

    #[test]
    fn php_ini_sets_jakarta_timezone() {
        assert!(render_php_ini(Path::new("/x")).contains("Asia/Jakarta"));
    }
}
