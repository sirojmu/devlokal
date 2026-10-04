# Spike: membuktikan asumsi lintas platform

Tujuan: dalam beberapa jam, tahu apakah PHP + nginx + MariaDB bisa dijalankan dari arsip portabel
di Windows dan Linux lewat `devlokal`. Jangan membangun fitur baru sebelum ini jelas.

## Windows

1. Unduh dari sumber resmi (jangan dari mirror pihak ketiga):
   - nginx: https://nginx.org/en/download.html (zip Windows)
   - PHP (NTS, x64): https://windows.php.net/download/
   - MariaDB (zip tanpa installer): https://mariadb.org/download/
2. Hitung checksum: `Get-FileHash <berkas> -Algorithm SHA256` dan cocokkan dengan halaman resmi bila tersedia.
3. Pasang:
   ```
   devlokal import nginx-<versi>   <nginx.zip> --strip 1
   devlokal import php-<versi>     <php.zip>
   devlokal import mariadb-<versi> <mariadb.zip> --strip 1
   devlokal doctor
   ```
   Atur `--strip` sesuai isi arsip (lihat apakah ada satu folder induk).
4. `devlokal new halo` → `devlokal sync` (Administrator) → `devlokal up` → buka `http://halo.test`.
5. Catat: apakah `php-cgi.exe` jalan tanpa DLL tambahan (VC++ runtime)? Apakah `mysql_install_db.exe`
   menghasilkan datadir dan `mariadbd` mau start dengan `my.ini` buatan devlokal? Port 80 bentrok dengan apa?

## Linux

Sama seperti di atas, tetapi PHP dan MariaDB tidak punya zip portabel resmi seperti di Windows.
Pilihan yang perlu dinilai: build statis (mis. static-php-cli), paket distro yang disalin ke folder,
atau mewajibkan paket sistem. Catat mana yang paling layak dan apa konsekuensi lisensinya.

## Hasil yang ingin dicatat

- Berapa MB unduhan total dan berapa MB RAM saat idle (target: ringan).
- Isi `manifest/components.toml` yang valid (URL + sha256) untuk satu versi tiap komponen.
- Daftar masalah per OS → jadi issue di GitHub.
