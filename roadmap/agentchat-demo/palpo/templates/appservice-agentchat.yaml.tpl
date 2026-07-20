id: agentchat-matrix-appservice
url: "{{APPSERVICE_URL}}"
as_token: "{{AS_TOKEN}}"
hs_token: "{{HS_TOKEN}}"
sender_localpart: "{{SENDER_LOCALPART}}"
rate_limited: false

namespaces:
  users:
    - exclusive: true
      regex: "@{{SENDER_LOCALPART}}:{{SERVER_REGEX}}"
    - exclusive: true
      regex: "@ac_.*:{{SERVER_REGEX}}"
  aliases: []
  rooms: []

