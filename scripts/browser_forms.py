"""Parse server-rendered form values for browser contract acceptance."""
from html.parser import HTMLParser
class Forms(HTMLParser):
 def __init__(self,html):
  super().__init__();self.forms=[];self.current=None;self.select=None;self.textarea=None;self.feed(html)
 def handle_starttag(self,tag,attrs):
  attrs=dict(attrs)
  if tag=='form':self.current={'action':attrs.get('action',''),'method':attrs.get('method','GET').upper(),'values':{}};self.forms.append(self.current)
  if self.current is None:return
  name=attrs.get('name')
  if tag=='input' and name and attrs.get('type') not in ['submit','checkbox','radio']:self.current['values'][name]=attrs.get('value','')
  if tag=='input' and name and 'checked' in attrs:self.current['values'][name]=attrs.get('value','on')
  if tag=='select':self.select=name
  if tag=='option' and self.select:
   if self.select not in self.current['values'] or 'selected' in attrs:self.current['values'][self.select]=attrs.get('value','')
  if tag=='textarea':self.textarea=name;self.current['values'][name]=''
 def handle_data(self,data):
  if self.current is not None and self.textarea:self.current['values'][self.textarea]+=data
 def handle_endtag(self,tag):
  if tag=='form':self.current=None
  if tag=='select':self.select=None
  if tag=='textarea':self.textarea=None
 def form(self,field):return next(form for form in self.forms if field in form['values'])['values']
