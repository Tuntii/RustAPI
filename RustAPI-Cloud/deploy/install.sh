#!/usr/bin/env bash
# RustAPI Cloud — production install on Ubuntu/Debian
# Usage: DOMAIN=api.example.com JWT_SECRET=... GITHUB_CLIENT_ID=... GITHUB_CLIENT_SECRET=... ./install.sh
set -euo pipefail

DOMAIN="${DOMAIN:-rustapi.tunayinbayramharcligi.com}"
CLOUD_PORT="${CLOUD_PORT:-3002}"
APP_ROOT="${APP_ROOT:-/opt/rustapi}"
CLOUD_DIR="${APP_ROOT}/RustAPI-Cloud"
SERVICE_USER="${SERVICE_USER:-rustapi}"
STORAGE_ROOT="/var/lib/rustapi-cloud/storage"
NGINX_MAP_DIR="/etc/nginx/rustapi-deploy-map.d"

echo "==> RustAPI Cloud install (domain: ${DOMAIN})"

if [[ "$(id -u)" -ne 0 ]]; then
  echo "Run as root." >&2
  exit 1
fi

echo "==> Remove previous release"
systemctl stop rustapi-cloud 2>/dev/null || true
systemctl disable rustapi-cloud 2>/dev/null || true
rm -f /etc/systemd/system/rustapi-cloud.service
rm -f /etc/nginx/sites-enabled/rustapi-cloud /etc/nginx/sites-available/rustapi-cloud
rm -f /etc/nginx/sites-enabled/rustapi-apps /etc/nginx/sites-available/rustapi-apps
rm -rf "${NGINX_MAP_DIR}"
rm -rf /opt/rustapi-cloud /var/www/rustapi-cloud /srv/rustapi-cloud
docker rm -f rustapi-cloud-db 2>/dev/null || true

echo "==> Install system packages"
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq \
  ca-certificates curl git build-essential pkg-config libssl-dev \
  nginx certbot python3-certbot-nginx \
  docker.io docker-compose-plugin postgresql-client

systemctl enable --now docker
systemctl enable --now nginx

if ! id "${SERVICE_USER}" &>/dev/null; then
  useradd --system --home "${APP_ROOT}" --shell /usr/sbin/nologin "${SERVICE_USER}"
fi

if ! command -v cargo &>/dev/null; then
  echo "==> Install Rust toolchain"
  su - "${SERVICE_USER}" -c 'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y'
fi

if [[ ! -f "${CLOUD_DIR}/Cargo.toml" ]]; then
  echo "==> Clone repository (or upload RustAPI tree to ${APP_ROOT} first)"
  mkdir -p "${APP_ROOT}"
  git clone --depth 1 https://github.com/Tuntii/RustAPI.git "${APP_ROOT}"
fi
chown -R "${SERVICE_USER}:${SERVICE_USER}" "${APP_ROOT}"

cd "${CLOUD_DIR}"

echo "==> Start Postgres"
docker compose up -d
for i in $(seq 1 30); do
  if docker compose exec -T db pg_isready -U rustapi -d rustapi_cloud &>/dev/null; then
    break
  fi
  sleep 2
done

echo "==> Apply migrations"
for f in migrations/*.sql; do
  echo "  - $(basename "$f")"
  docker compose exec -T db psql -U rustapi -d rustapi_cloud -v ON_ERROR_STOP=1 < "$f"
done

JWT_SECRET="${JWT_SECRET:-$(openssl rand -hex 32)}"
GITHUB_CLIENT_ID="${GITHUB_CLIENT_ID:-REPLACE_ME}"
GITHUB_CLIENT_SECRET="${GITHUB_CLIENT_SECRET:-REPLACE_ME}"

mkdir -p "${STORAGE_ROOT}"
chown -R "${SERVICE_USER}:${SERVICE_USER}" "${STORAGE_ROOT}"

cat > "${CLOUD_DIR}/.env" <<EOF
DATABASE_URL=postgres://rustapi:rustapi@localhost:5435/rustapi_cloud
HOST=127.0.0.1
PORT=${CLOUD_PORT}
JWT_SECRET=${JWT_SECRET}
GITHUB_CLIENT_ID=${GITHUB_CLIENT_ID}
GITHUB_CLIENT_SECRET=${GITHUB_CLIENT_SECRET}
GITHUB_REDIRECT_URI=https://${DOMAIN}/auth/callback
STORAGE_ROOT=${STORAGE_ROOT}
DEPLOY_PUBLIC_HOST=${DOMAIN}
DEPLOY_URL_SCHEME=https
NGINX_DEPLOY_MAP_DIR=${NGINX_MAP_DIR}
RUST_LOG=rustapi_cloud=info,info
EOF
chmod 600 "${CLOUD_DIR}/.env"
chown "${SERVICE_USER}:${SERVICE_USER}" "${CLOUD_DIR}/.env"

echo "==> Build rustapi-cloud (release)"
su - "${SERVICE_USER}" -c "cd ${CLOUD_DIR} && source \$HOME/.cargo/env && cargo build --release"

install -m 0644 "${CLOUD_DIR}/deploy/rustapi-cloud.service" /etc/systemd/system/rustapi-cloud.service
sed -i "s|__APP_ROOT__|${APP_ROOT}|g" /etc/systemd/system/rustapi-cloud.service
sed -i "s|__SERVICE_USER__|${SERVICE_USER}|g" /etc/systemd/system/rustapi-cloud.service

mkdir -p "${NGINX_MAP_DIR}"
chown "${SERVICE_USER}:${SERVICE_USER}" "${NGINX_MAP_DIR}"
chmod 0755 "${NGINX_MAP_DIR}"
echo "${SERVICE_USER} ALL=(root) NOPASSWD: /usr/sbin/nginx -s reload" > /etc/sudoers.d/rustapi-nginx-reload
chmod 0440 /etc/sudoers.d/rustapi-nginx-reload

install -m 0644 "${CLOUD_DIR}/deploy/nginx-rustapi-cloud.conf" /etc/nginx/sites-available/rustapi-cloud
sed -i "s|__DOMAIN__|${DOMAIN}|g" /etc/nginx/sites-available/rustapi-cloud
sed -i "s|__CLOUD_PORT__|${CLOUD_PORT}|g" /etc/nginx/sites-available/rustapi-cloud
ln -sf /etc/nginx/sites-available/rustapi-cloud /etc/nginx/sites-enabled/rustapi-cloud

install -m 0644 "${CLOUD_DIR}/deploy/nginx-rustapi-apps.conf" /etc/nginx/sites-available/rustapi-apps
sed -i "s|__DOMAIN__|${DOMAIN}|g" /etc/nginx/sites-available/rustapi-apps
sed -i "s|__NGINX_MAP_DIR__|${NGINX_MAP_DIR}|g" /etc/nginx/sites-available/rustapi-apps
ln -sf /etc/nginx/sites-available/rustapi-apps /etc/nginx/sites-enabled/rustapi-apps
rm -f /etc/nginx/sites-enabled/default
nginx -t
systemctl reload nginx

systemctl daemon-reload
systemctl enable --now rustapi-cloud

if [[ "${GITHUB_CLIENT_ID}" == "REPLACE_ME" ]]; then
  echo "WARN: Set real GITHUB_CLIENT_ID / GITHUB_CLIENT_SECRET in ${CLOUD_DIR}/.env then: systemctl restart rustapi-cloud"
fi

if certbot certificates 2>/dev/null | grep -q "${DOMAIN}"; then
  certbot renew --quiet || true
else
  echo "==> Obtain TLS certificate (requires DNS A record -> this server)"
  certbot --nginx -d "${DOMAIN}" --non-interactive --agree-tos -m "admin@${DOMAIN#*.}" --redirect || \
    echo "WARN: certbot failed — point ${DOMAIN} A record to this server, then run: certbot --nginx -d ${DOMAIN}"
  certbot --nginx -d "*.${DOMAIN}" --non-interactive --agree-tos -m "admin@${DOMAIN#*.}" --redirect 2>/dev/null || \
    echo "WARN: wildcard TLS for *.${DOMAIN} needs DNS-01 — run: certbot certonly --manual --preferred-challenges dns -d '*.${DOMAIN}'"
fi

echo "==> Health check"
sleep 3
curl -fsS "http://127.0.0.1:${CLOUD_PORT}/health" && echo
echo "DONE: https://${DOMAIN}/health"
echo "User apps: https://{project}-{user8}.${DOMAIN}"
echo "DNS: A ${DOMAIN} -> server IP, A *.${DOMAIN} -> server IP (wildcard)"
echo "CLI login: cargo rustapi login --cloud-url https://${DOMAIN}"