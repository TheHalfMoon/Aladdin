# Quntal open-source relay (SG-000057). Build from the repository root:
#   docker build -f apps/quntal-relay/Dockerfile -t quntal-relay .
# Base image pinned by digest (node 24.19.0, Alpine 3.23).
FROM node:24.19.0-alpine3.23@sha256:244cc2b53f46f9e876304391d17682b0ddae9ac33491f4857e25e35a36ba7995 AS build
WORKDIR /src
COPY package.json package-lock.json ./
COPY apps/quntal-mcp/package.json apps/quntal-mcp/package.json
COPY apps/quntal-relay/package.json apps/quntal-relay/package.json
RUN npm ci --ignore-scripts
COPY apps/quntal-mcp apps/quntal-mcp
COPY apps/quntal-relay apps/quntal-relay
RUN npm run build --workspace @quntal/mcp && npm run build --workspace @quntal/relay \
 && npm prune --omit=dev --ignore-scripts

FROM node:24.19.0-alpine3.23@sha256:244cc2b53f46f9e876304391d17682b0ddae9ac33491f4857e25e35a36ba7995
ENV NODE_ENV=production
WORKDIR /app
COPY --from=build /src/node_modules ./node_modules
COPY --from=build /src/apps/quntal-mcp/package.json ./apps/quntal-mcp/package.json
COPY --from=build /src/apps/quntal-mcp/dist ./apps/quntal-mcp/dist
COPY --from=build /src/apps/quntal-relay/package.json ./apps/quntal-relay/package.json
COPY --from=build /src/apps/quntal-relay/dist ./apps/quntal-relay/dist
RUN mkdir -p /var/lib/quntal-relay && chown node:node /var/lib/quntal-relay
USER node
EXPOSE 8787
HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://127.0.0.1:8787/healthz || exit 1
ENTRYPOINT ["node", "/app/apps/quntal-relay/dist/main.js"]
CMD ["--config", "/etc/quntal-relay/relay.json"]
