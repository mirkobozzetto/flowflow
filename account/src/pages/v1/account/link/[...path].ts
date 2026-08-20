import type { APIRoute } from "astro";
import { forward } from "../../../../lib/proxy";

export const prerender = false;

export const ALL: APIRoute = (ctx) => forward(ctx.request, ctx.clientAddress);
