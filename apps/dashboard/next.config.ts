import type { NextConfig } from "next";

const nextConfig: NextConfig = {
	output: "standalone",
	async rewrites() {
		const internalApiUrl = process.env.NANOSCALE_INTERNAL_API_URL ?? "http://127.0.0.1:4000";

		return [
			{
				source: "/api/:path*",
				destination: `${internalApiUrl}/api/:path*`,
			},
		];
	},
};

export default nextConfig;
