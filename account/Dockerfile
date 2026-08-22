FROM oven/bun:1 AS build
WORKDIR /app
COPY package.json bun.lock* ./
RUN bun install --frozen-lockfile
COPY . .
RUN bun run build

FROM node:22-slim
WORKDIR /app
ENV HOST=0.0.0.0 PORT=4321 NODE_ENV=production
# The standalone adapter still requires externalized packages at runtime
# (@astrojs/internal-helpers and friends), so node_modules must ship too.
COPY --from=build /app/node_modules ./node_modules
COPY --from=build /app/package.json ./
COPY --from=build /app/dist ./dist
EXPOSE 4321
CMD ["node", "./dist/server/entry.mjs"]
