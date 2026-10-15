# Orcel open-source relay (SG-000057). Build from the repository root:
#   docker build -f apps/orcel-relay/Dockerfile -t orcel-relay .
# Base image pinned by digest (node 24.19.0, Alpine 3.23).
FROM node:24.19.0-alpine3.23@sha256:244cc2b53f46f9e876304391d17682b0ddae9ac33491f4857e25e35a36ba7995 AS build
WORKDIR /src
COPY package.json package-lock.json ./
COPY apps/orcel-mcp/package.json apps/orcel-mcp/package.json
COPY apps/orcel-relay/package.json apps/orcel-relay/package.json
RUN npm ci --ignore-scripts
COPY apps/orcel-mcp apps/orcel-mcp
COPY apps/orcel-relay apps/orcel-relay
RUN npm run build --workspace @orcel/mcp && npm run build --workspace @orcel/relay \
 && npm prune --omit=dev --ignore-scripts

FROM node:24.19.0-alpine3.23@sha256:244cc2b53f46f9e876304391d17682b0ddae9ac33491f4857e25e35a36ba7995
ENV NODE_ENV=production
WORKDIR /app
COPY --from=build /src/node_modules ./node_modules
COPY --from=build /src/apps/orcel-mcp/package.json ./apps/orcel-mcp/package.json
COPY --from=build /src/apps/orcel-mcp/dist ./apps/orcel-mcp/dist
COPY --from=build /src/apps/orcel-relay/package.json ./apps/orcel-relay/package.json
COPY --from=build /src/apps/orcel-relay/dist ./apps/orcel-relay/dist
RUN mkdir -p /var/lib/orcel-relay && chown node:node /var/lib/orcel-relay
USER node
EXPOSE 8787
HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://127.0.0.1:8787/healthz || exit 1
ENTRYPOINT ["node", "/app/apps/orcel-relay/dist/main.js"]
CMD ["--config", "/etc/orcel-relay/relay.json"]
