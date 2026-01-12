<?php

namespace App\Http\Controllers;

use Illuminate\Routing\Controller;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\DB;
use Carbon\Carbon;

class BenchmarkController extends Controller
{
    public function plaintext()
    {
        return response('Hello, World!', 200, ['Content-Type' => 'text/plain']);
    }

    public function json(Request $request)
    {
        $orders = $request->json()->all();
        
        $processedOrders = 0;
        $results = [];
        $categoryStats = [];

        foreach ($orders as $order) {
            if (($order['status'] ?? '') !== 'completed') {
                continue;
            }

            $processedOrders++;

            $country = $order['country'] ?? 'unknown';
            $amount = $order['amount'] ?? 0;
            
            if (isset($results[$country])) {
                $results[$country] += $amount;
            } else {
                $results[$country] = $amount;
            }
            
            if (isset($order['items']) && is_array($order['items'])) {
                foreach ($order['items'] as $item) {
                     $category = $item['category'] ?? 'unknown';
                     $quantity = $item['quantity'] ?? 0;
                     
                     if (isset($categoryStats[$category])) {
                         $categoryStats[$category] += $quantity;
                     } else {
                         $categoryStats[$category] = $quantity;
                     }
                }
            }
        }
        
        // Ensure maps are objects even if empty, but PHP arrays serialize to [] if empty list or {} if associative.
        // We'll trust typical serialization but might need explicit object cast if empty.
        // Spec implies maps, so {} is better than [].
        
        return response()->json([
            'processedOrders' => $processedOrders,
            'results' => (object)$results,
            'categoryStats' => (object)$categoryStats,
        ]);
    }
    
    public function dbUserProfile($email)
    {
        $user = DB::table('users')->where('email', $email)->first();
        
        if (!$user) {
            abort(404, 'User not found');
        }
        
        $trending = DB::table('posts')
            ->select('id', 'title', 'content', 'views', 'created_at')
            ->orderBy('views', 'desc')
            ->limit(5)
            ->get();
            
        $now = now();
        DB::table('users')->where('id', $user->id)->update(['last_login' => $now]);
        
        $posts = DB::table('posts')
            ->select('id', 'title', 'content', 'views', 'created_at')
            ->where('user_id', $user->id)
            ->orderBy('created_at', 'desc')
            ->limit(10)
            ->get();
            
        $mapPost = function($p) {
            return [
                'id' => $p->id,
                'title' => $p->title,
                'content' => $p->content,
                'views' => $p->views,
                'createdAt' => $p->created_at ? Carbon::parse($p->created_at)->format('Y-m-d\TH:i:s\Z') : null,
            ];
        };
        
        // Decode settings if string (Postgres JSON might be auto-decoded by driver or not, with DB facade it often returns string for JSON columns without Casts)
        $settings = $user->settings;
        if (is_string($settings)) {
             $settings = json_decode($settings);
        }

        return response()->json([
            'username' => $user->username,
            'email' => $user->email,
            'createdAt' => Carbon::parse($user->created_at)->format('Y-m-d\TH:i:s\Z'),
            'lastLogin' => $now->format('Y-m-d\TH:i:s\Z'),
            'settings' => $settings,
            'posts' => $posts->map($mapPost),
            'trending' => $trending->map($mapPost)
        ]);
    }
}
