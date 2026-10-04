# Qdral open-source relay (SG-000057). Build from the repository root:
#   docker build -f apps/qdral-relay/Dockerfile -t qdral-relay .
# Base image pinned by digest (node 24.19.0, Alpine 3.23).
FROM node:24.19.0-alpine3.23@sha256:244cc2b53f46f9e876304391d17682b0ddae9ac33491f4857e25e35a36ba7995 AS build
WORKDIR /src
COPY package.json package-lock.json ./
COPY apps/qdral-mcp/package.json apps/qdral-mcp/package.json
COPY apps/qdral-relay/package.json apps/qdral-relay/package.json
RUN npm ci --ignore-scripts
COPY apps/qdral-mcp apps/qdral-mcp
COPY apps/qdral-relay apps/qdral-relay
RUN npm run build --workspace @qdral/mcp && npm run build --workspace @qdral/relay \
 && npm prune --omit=dev --ignore-scripts

FROM node:24.19.0-alpine3.23@sha256:244cc2b53f46f9e876304391d17682b0ddae9ac33491f4857e25e35a36ba7995
ENV NODE_ENV=production
WORKDIR /app
COPY --from=build /src/node_modules ./node_modules
COPY --from=build /src/apps/qdral-mcp/package.json ./apps/qdral-mcp/package.json
COPY --from=build /src/apps/qdral-mcp/dist ./apps/qdral-mcp/dist
COPY --from=build /src/apps/qdral-relay/package.json ./apps/qdral-relay/package.json
COPY --from=build /src/apps/qdral-relay/dist ./apps/qdral-relay/dist
RUN mkdir -p /var/lib/qdral-relay && chown node:node /var/lib/qdral-relay
USER node
EXPOSE 8787
HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://127.0.0.1:8787/healthz || exit 1
ENTRYPOINT ["node", "/app/apps/qdral-relay/dist/main.js"]
CMD ["--config", "/etc/qdral-relay/relay.json"]
