package com.wfb.spring.api.service;

import com.wfb.spring.api.dto.ServletDef;
import com.wfb.spring.api.dto.WebAppPayload;
import org.springframework.stereotype.Service;

@Service
public class JsonReplaceService {
    public WebAppPayload replaceServletName(WebAppPayload root, String from, String to) {
        if (root == null || root.getWebApp() == null || root.getWebApp().getServlet() == null) {
            return root;
        }

        for (ServletDef servlet : root.getWebApp().getServlet()) {
            if (servlet != null && from.equals(servlet.getServletName())) {
                servlet.setServletName(to);
            }
        }
        return root;
    }
}
