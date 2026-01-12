<?php

declare(strict_types=1);

namespace App\Controller;

use Hyperf\DbConnection\Db;
use Hyperf\HttpServer\Contract\RequestInterface;
use Hyperf\HttpServer\Contract\ResponseInterface;
use function Hyperf\Coroutine\parallel;

class DbController
{
    public function complex(string $email, ResponseInterface $response)
    {
        try {
            // Hyperf doesn't support async DB queries perfectly within the same coroutine easily for simple usage,
            // but we can use coroutines or parallel execution if needed.
            // For simplicity and typical usage in Hyperf which is coroutine-based blocking style (synchronous code, async execution):
            
            // 1. Get User
            $user = Db::table('users')->where('email', $email)->select(['id', 'username', 'email', 'created_at as createdAt', 'last_login as lastLogin', 'settings'])->first();

            if (!$user) {
                return $response->withStatus(404)->json(['error' => 'User not found']);
            }
            
            // Decode settings from JSON string if Db returns it as string (depends on driver/config)
            // Hyperf/PDO usually returns JSON columns as strings.
            if (is_string($user->settings)) {
                $user->settings = json_decode($user->settings);
            }

            // 2. Get Trending Posts (Global)
            // Ideally we could run these in parallel with Co::batch or wait group.
            // Let's optimize with Co::batch later if needed, but sequential is standard for "simple" implementations. 
            // However, spec recommends parallel. Hyperf makes parallel easy.
            
            // Let's rewrite with parallel() for User + Trending
            // But wait, we need User ID for the next step. So we can only parallelize User + Trending.
            
            // Re-fetching User and Trending in Parallel
             [$user, $trending] = parallel([
                function () use ($email) {
                    return Db::table('users')->where('email', $email)->select(['id', 'username', 'email', 'created_at as createdAt', 'last_login as lastLogin', 'settings'])->first();
                },
                function () {
                    return Db::table('posts')->orderBy('views', 'desc')->limit(5)->get(['id', 'title', 'content', 'views', 'created_at as createdAt']);
                }
            ]);

            if (!$user) {
                return $response->withStatus(404)->json(['error' => 'User not found']);
            }

            if (is_string($user->settings)) {
                $user->settings = json_decode($user->settings);
            }

            // 3. Update Last Login & Get User Posts (Parallel)
            [$update, $posts] = parallel([
                function () use ($user) {
                    // Update and Return is tricky in standard SQL across DBs, but Postgres supports RETURNING.
                    // Hyperf Query Builder update doesn't support RETURNING easily without raw.
                    // Let's do update then select or use raw.
                    
                    // Native efficient way:
                    $newTime = date('Y-m-d H:i:s'); // PHP time is sufficient or use DB NOW()
                    // We need the DB's NOW() usually to be exact? 
                    // Spec says "Update the user's last_login field to the current timestamp (NOW())"
                    
                    Db::update("UPDATE users SET last_login = NOW() WHERE id = ?", [$user->id]);
                    // We need the value we just set? Or just return it. 
                    // To be safe and compliant "RETURNING last_login", let's use select or assume it worked.
                    // Spec: "Map the results... lastLogin is present (indicating update)"
                    
                    // Let's fetch it back to be sure or use NOW() from PHP
                    return Db::table('users')->where('id', $user->id)->value('last_login');
                },
                function () use ($user) {
                    return Db::table('posts')
                        ->where('user_id', $user->id)
                        ->orderBy('created_at', 'desc')
                        ->limit(10)
                        ->get(['id', 'title', 'content', 'views', 'created_at as createdAt']);
                }
            ]);
            
            $user->lastLogin = $update;

            // Structure response
            $result = (array)$user;
            $result['posts'] = $posts;
            $result['trending'] = $trending;

            return $response->json($result);

        } catch (\Throwable $e) {
            error_log((string)$e);
            return $response->withStatus(500)->json(['error' => 'Internal Server Error', 'details' => $e->getMessage()]);
        }
    }
}
