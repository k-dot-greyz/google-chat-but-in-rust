import { dispatch } from "../../lib/hull.mjs";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

async function handle(request) {
  return dispatch(request);
}

export const GET = handle;
export const POST = handle;
export const PUT = handle;
export const PATCH = handle;
export const DELETE = handle;
export const HEAD = handle;
export const OPTIONS = handle;
