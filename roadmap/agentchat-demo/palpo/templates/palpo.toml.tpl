server_name = "{{SERVER_NAME}}"
allow_registration = true
yes_i_am_very_very_sure_i_want_an_open_registration_server_prone_to_abuse = false
registration_token = "{{REGISTRATION_TOKEN}}"
rc_registration = { per_second = 1.0, burst = 10 }
enable_admin_room = true
appservice_registration_dir = "/var/palpo/appservices"

[[listeners]]
address = "0.0.0.0:8008"

[logger]
format = "pretty"

[db]
url = "postgres://{{DB_USER}}:{{DB_PASSWORD}}@{{DB_HOST}}:{{DB_PORT}}/{{DB_NAME}}"
pool_size = 10

[well_known]
server = "{{SERVER_NAME}}"
client = "{{PUBLIC_URL}}"
