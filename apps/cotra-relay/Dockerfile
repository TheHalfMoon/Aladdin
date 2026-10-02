# Cotra open-source relay (SG-000057). Build from the repository root:
#   docker build -f apps/cotra-relay/Dockerfile -t cotra-relay .
# Base image pinned by digest (node 24.19.0, Alpine 3.23).
FROM node:24.19.0-alpine3.23@sha256:244cc2b53f46f9e876304391d17682b0ddae9ac33491f4857e25e35a36ba7995 AS build
WORKDIR /src
COPY package.json package-lock.json ./
COPY apps/cotra-mcp/package.json apps/cotra-mcp/package.json
COPY apps/cotra-relay/package.json apps/cotra-relay/package.json
RUN npm ci --ignore-scripts
COPY apps/cotra-mcp apps/cotra-mcp
COPY apps/cotra-relay apps/cotra-relay
RUN npm run build --workspace @cotra/mcp && npm run build --workspace @cotra/relay \
 && npm prune --omit=dev --ignore-scripts

FROM node:24.19.0-alpine3.23@sha256:244cc2b53f46f9e876304391d17682b0ddae9ac33491f4857e25e35a36ba7995
ENV NODE_ENV=production
WORKDIR /app
COPY --from=build /src/node_modules ./node_modules
COPY --from=build /src/apps/cotra-mcp/package.json ./apps/cotra-mcp/package.json
COPY --from=build /src/apps/cotra-mcp/dist ./apps/cotra-mcp/dist
COPY --from=build /src/apps/cotra-relay/package.json ./apps/cotra-relay/package.json
COPY --from=build /src/apps/cotra-relay/dist ./apps/cotra-relay/dist
RUN mkdir -p /var/lib/cotra-relay && chown node:node /var/lib/cotra-relay
USER node
EXPOSE 8787
HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://127.0.0.1:8787/healthz || exit 1
ENTRYPOINT ["node", "/app/apps/cotra-relay/dist/main.js"]
CMD ["--config", "/etc/cotra-relay/relay.json"]
